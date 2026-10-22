"""检查真实静态 HTML，不用 JavaScript 翻译结果冒充多语言抓取通过。"""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit, unquote
import json
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent / 'dist'
ORIGIN = 'https://focroom.cc'
LANGUAGES = {'zh': 'zh-CN', 'en': 'en', 'ja': 'ja', 'ko': 'ko'}
PAGES = ('home', 'workflow', 'guide', 'privacy', 'plugins')


def route(page, language):
    return ('/' if language == 'zh' else f'/{language}/') + ('' if page == 'home' else f'{page}/')


class Document(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.ids, self.refs, self.links, self.meta = [], [], [], {}
        self.lang = self.title = self.schema = ''
        self.h1 = 0
        self.body_text = []
        self._title = self._schema = self._body = False
        self._skip = 0
        self._skip_end = []
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.append(attrs['id'])
        if tag == 'html':
            self.lang = attrs.get('lang')
        if tag == 'h1':
            self.h1 += 1
        if tag == 'meta':
            self.meta[attrs.get('name', attrs.get('property'))] = attrs.get('content')
        if tag == 'link':
            self.links.append(attrs)
        for key in ('href', 'src'):
            if key in attrs:
                self.refs.append(attrs[key])
        if tag == 'title':
            self._title = True
        if tag == 'body':
            self._body = True
        if tag == 'script' and attrs.get('type') == 'application/ld+json':
            self._schema = True
        if tag in ('script', 'style', 'select') or 'data-no-translate' in attrs:
            self._skip += 1
            self._skip_end.append(tag)

    def handle_endtag(self, tag):
        if tag == 'title':
            self._title = False
        if tag == 'script':
            self._schema = False
        if tag == 'body':
            self._body = False
        if self._skip_end and tag == self._skip_end[-1]:
            self._skip -= 1
            self._skip_end.pop()

    def handle_data(self, data):
        if self._title:
            self.title += data
        if self._schema:
            self.schema += data
        if self._body and not self._skip:
            self.body_text.append(data)


documents = {}
errors = []
references = 0
for language, code in LANGUAGES.items():
    for page in PAGES:
        urlpath = route(page, language)
        file = ROOT / urlpath.lstrip('/') / 'index.html'
        text = file.read_text()
        doc = Document(text)
        documents[urlpath] = doc
        if doc.lang != code or doc.h1 != 1 or len(doc.ids) != len(set(doc.ids)):
            errors.append(f'{urlpath}: lang, h1 or duplicate id error')
        canonical = [x.get('href') for x in doc.links if x.get('rel') == 'canonical']
        if canonical != [ORIGIN + urlpath]:
            errors.append(f'{urlpath}: wrong canonical {canonical}')
        alternates = {x.get('hreflang'): x.get('href') for x in doc.links if x.get('rel') == 'alternate'}
        expected = {value: ORIGIN + route(page, key) for key, value in LANGUAGES.items()}
        expected['x-default'] = ORIGIN + route(page, 'zh')
        if alternates != expected:
            errors.append(f'{urlpath}: incomplete hreflang cluster')
        for name, expected in [('og:url', ORIGIN + urlpath), ('og:title', doc.title), ('twitter:title', doc.title), ('og:description', doc.meta.get('description'))]:
            if not expected or doc.meta.get(name) != expected:
                errors.append(f'{urlpath}: inconsistent {name}')
        if not doc.meta.get('description') or doc.meta.get('robots') != 'index,follow,max-image-preview:large':
            errors.append(f'{urlpath}: missing description/index metadata')
        data = json.loads(doc.schema)
        webpage = next(x for x in data['@graph'] if x['@type'] == 'WebPage')
        if webpage['url'] != ORIGIN + urlpath or webpage['inLanguage'] != code:
            errors.append(f'{urlpath}: schema locale mismatch')
        visible = re.sub(r'\s+', ' ', ' '.join(doc.body_text))
        if language == 'en' and re.search(r'[\u3400-\u9fff]', visible):
            errors.append(f'{urlpath}: untranslated Chinese body text')
        for item in data['@graph']:
            if item['@type'] == 'FAQPage':
                for q in item['mainEntity']:
                    if q['name'] not in visible or q['acceptedAnswer']['text'] not in visible:
                        errors.append(f'{urlpath}: FAQ not backed by visible text')
            if any(key in item for key in ('aggregateRating', 'offers', 'review')):
                errors.append(f'{urlpath}: unverified pricing or review claim')
        if page == 'home' and ('data-open="access-dialog"' in text or '<form' in text or 'download/download/' in text):
            errors.append(f'{urlpath}: recruitment/form/broken download')

for urlpath, doc in documents.items():
    for value in doc.refs:
        url = urlsplit(value)
        if url.scheme or url.netloc:
            continue
        references += 1
        target = unquote(url.path) if url.path else urlpath
        if not target.startswith('/'):
            errors.append(f'{urlpath}: non-root-relative local URL {value}')
            continue
        file = (ROOT / target.lstrip('/')).resolve()
        if not file.is_relative_to(ROOT.resolve()):
            errors.append(f'{urlpath}: directory escape {value}')
            continue
        if file.is_dir():
            file /= 'index.html'
        if not file.is_file():
            errors.append(f'{urlpath}: missing target {value}')
        elif url.fragment:
            dest = documents.get(target) or Document(file.read_text())
            if unquote(url.fragment) not in dest.ids:
                errors.append(f'{urlpath}: missing anchor {value}')

sitemap = ET.fromstring((ROOT / 'sitemap.xml').read_text())
locations = [x.text for x in sitemap.findall('{*}url/{*}loc')]
if len(locations) != len(LANGUAGES) * len(PAGES) or set(locations) != {ORIGIN + key for key in documents}:
    errors.append('sitemap: canonical URL coverage mismatch')
if 'Sitemap: https://focroom.cc/sitemap.xml' not in (ROOT / 'robots.txt').read_text():
    errors.append('robots: missing sitemap')
if 'noindex' not in (ROOT / '404.html').read_text() or 'X-Robots-Tag: noindex' not in (ROOT / '_headers').read_text():
    errors.append('missing not-found/manual indexing controls')
for language in LANGUAGES:
    text = (ROOT / ('' if language == 'zh' else language) / 'llms.txt').read_text()
    for page in PAGES:
        if ORIGIN + route(page, language) not in text:
            errors.append(f'llms/{language}: missing page {page}')
print(json.dumps({'staticPages': len(documents), 'languages': list(LANGUAGES), 'localReferences': references, 'sitemapURLs': len(locations), 'errors': errors}, ensure_ascii=False, indent=2))
raise SystemExit(bool(errors))
