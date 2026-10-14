from pathlib import Path
from playwright.sync_api import sync_playwright
from zipfile import ZipFile
html=Path(__file__).parent/'demo-standalone.html'
errors=[]
with sync_playwright() as p:
 b=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox'])
 page=b.new_page(viewport={'width':1440,'height':900},accept_downloads=True)
 page.on('pageerror', lambda err: errors.append(str(err)))
 page.set_content(html.read_text(),wait_until='load')
 assert page.locator('.tile').count()==28
 page.locator('[data-action="run-ai"]').first.click()
 assert page.locator('.tile').count()==11
 assert page.locator('.dock-images button').count()==0
 page.locator('.tile').first.click()
 page.keyboard.press('k')
 assert page.locator('.dock-images button').count()==1
 page.locator('[data-action="accept-suggestions"]').first.click()
 assert page.locator('.dock-images button').count()==11
 page.locator('[data-action="view-seq"]').first.click()
 assert page.locator('.seq-card').count()==11
 page.locator('[data-action="compare"]').first.click()
 assert page.locator('.compare-cols img').count()==2
 page.locator('[data-action="prefer"]').first.click()
 assert page.locator('.modal-bg').count()==0
 page.locator('[data-action="edit-project"]').first.click()
 page.locator('#project-title').fill('新摄影项目')
 page.locator('[data-action="save-project"]').first.click()
 assert page.locator('h1').inner_text()=='新摄影项目'
 page.locator('[data-action="export"]').first.click()
 with page.expect_download() as download_info:
  page.locator('[data-action="download-json"]').click()
 download=download_info.value
 assert download.suggested_filename=='between-frames-project.json'
 assert not errors,errors
 page.set_viewport_size({'width':390,'height':844})
 page.locator('[data-action="close-modal"]').first.click()
 assert page.locator('h1').count()==1
 page.screenshot(path=str(Path(__file__).parent/'screenshot-mobile.png'),full_page=True)
 b.close()
with ZipFile('/mnt/data/BetweenFrames-Agent-Development-Kit.zip') as z:
 assert z.testzip() is None
print('PASS: grid -> AI proposals -> human accept -> sequence -> pairwise -> project update -> export -> mobile')
print('PASS: distributable zip integrity')
