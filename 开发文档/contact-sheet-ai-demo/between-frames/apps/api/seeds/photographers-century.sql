-- K5 扩充：近百年摄影史核心摄影师（1930s–2020s）
-- 数据来源：各摄影师的博物馆馆藏记录、出版文献和艺术家基金会公开资料。
-- 每条含：拍摄格式（关键区分）、代表系列、所属流派。
-- 幂等：按 name_en UPSERT。
-- 置信标注：verified = 多来源交叉确认；imported = 单来源或待校审。

BEGIN;

-- ═══ 1930s–1940s 现代主义 / f/64 / 纪实开端 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Ansel Adams', '安塞尔·亚当斯', 1902, 1984, 'American',
  'Landscape photographer and environmentalist known for black-and-white images of the American West.',
  'Zone System co-developer; 8x10 view camera for Sierra Nevada work; meticulous darkroom printing.',
  ARRAY['Group f/64','Straight Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography, method_notes = EXCLUDED.method_notes;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Edward Weston', '爱德华·韦斯顿', 1886, 1958, 'American',
  'One of the most influential American photographers of the 20th century; master of the large-format camera.',
  '8x10 view camera; pepper and shell close-ups; Point Lobos landscapes; platinum and gelatin silver prints.',
  ARRAY['Group f/64','Straight Photography','Modernism'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Henri Cartier-Bresson', '亨利·卡蒂埃-布列松', 1908, 2004, 'French',
  'Father of modern photojournalism; co-founder of Magnum Photos.',
  '35mm Leica exclusively; "decisive moment"; never cropped his images; 50mm lens primary.',
  ARRAY['Humanist Photography','Street Photography','Photojournalism'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Walker Evans', '沃克·埃文斯', 1903, 1975, 'American',
  'Documentary photographer best known for Farm Security Administration work and American vernacular.',
  '8x10 view camera for FSA; Subway series with hidden 35mm camera; direct frontal approach.',
  ARRAY['Documentary Photography','FSA'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Dorothea Lange', '多萝西娅·兰格', 1895, 1965, 'American',
  'Documentary photographer whose portraits of displaced Americans humanized the Great Depression.',
  '4x5 Graflex Crown camera; Migrant Mother 1936; FSA assignment methodology.',
  ARRAY['Documentary Photography','FSA'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 1950s–1960s 战后 / 纽约派 / 新纪实 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Robert Doisneau', '罗伯特·杜瓦诺', 1912, 1994, 'French',
  'Humanist photographer celebrated for playful and tender images of Paris street life.',
  'Rolleiflex 6x6; Le Baiser de l''Hôtel de Ville (The Kiss) 1950; gentle observation.',
  ARRAY['Humanist Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Irving Penn', '欧文·佩恩', 1917, 2009, 'American',
  'Master of studio portraiture and still life; Vogue photographer for over 60 years.',
  '8x10 view camera in studio; corner poses; platinum-palladium prints; Worlds in a Small Room.',
  ARRAY['Studio Photography','Fashion Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Richard Avedon', '理查德·阿维顿', 1923, 2004, 'American',
  'Fashion and portrait photographer who revolutionized both genres with psychological intensity.',
  '8x10 Deardorff; white backdrop; In the American West series; large-format portrait method.',
  ARRAY['Portrait Photography','Fashion Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Garry Winogrand', '加里·维诺格兰德', 1928, 1984, 'American',
  'Prolific street photographer whose kinetic frames captured the social landscape of mid-century America.',
  '35mm Leica; wide-angle 28mm; tilted horizons; shot voraciously—thousands of undeveloped rolls at death.',
  ARRAY['Street Photography','New Documents'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Lee Friedlander', '李·弗里德兰德', 1934, NULL, 'American',
  'Photographer of the American "social landscape" known for complex, layered compositions.',
  '35mm Leica; reflections and shadows; street signs and urban fragments; self-portraits.',
  ARRAY['Street Photography','New Documents'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Helen Levitt', '海伦·莱维特', 1913, 2009, 'American',
  'Street photographer celebrated for lyrical images of children playing in New York City.',
  '35mm Leica with right-angle viewfinder; 1940s black-and-white and later color dye-transfer.',
  ARRAY['Street Photography','Documentary Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 1970s 新彩色 / 新地形 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Joel Meyerowitz', '乔尔·迈耶罗维茨', 1938, NULL, 'American',
  'Pioneer of color photography alongside Eggleston and Shore; street and landscape work.',
  'Started 35mm street; transitioned to 8x10 color view camera; Cape Light series.',
  ARRAY['New Color Photography','Street Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Robert Adams', '罗伯特·亚当斯', 1937, NULL, 'American',
  'Photographer of the American West documenting suburban encroachment on landscape.',
  '4x5 and 8x10 view cameras; The New West; Denver; What We Bought; plainspoken approach.',
  ARRAY['New Topographics'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Lewis Baltz', '刘易斯·巴尔茨', 1945, 2014, 'American',
  'Minimalist photographer of industrial parks and urban edge zones.',
  '8x10 view camera; The New Industrial Parks; stark geometry; New Topographics 1975.',
  ARRAY['New Topographics','Minimalism'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Nicholas Nixon', '尼古拉斯·尼克松', 1947, NULL, 'American',
  'Large-format photographer known for The Brown Sisters annual portraits and urban views.',
  '8x10 view camera; The Brown Sisters (1975–ongoing annual series); People with AIDS.',
  ARRAY['New Topographics','Portrait Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 日本摄影 1960s–2000s ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Daido Moriyama', '森山大道', 1938, NULL, 'Japanese',
  'Iconic Japanese street photographer known for grainy, high-contrast black-and-white images.',
  '35mm compact; are-bure-boke (grainy, blurry, out-of-focus); Stray Dog; Japan: A Photo Theater.',
  ARRAY['Provoke','Japanese Street Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Shomei Tomatsu', '东松照明', 1930, 2012, 'Japanese',
  'Leading postwar Japanese photographer documenting Nagasaki and American occupation.',
  '35mm; Nagasaki 11:02 1966; biting visual commentary; influenced Provoke generation.',
  ARRAY['Japanese Postwar Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Eikoh Hosoe', '细江英公', 1933, 2024, 'Japanese',
  'Avant-garde photographer whose collaborations with writers produced surreal masterpieces.',
  'Barakei (Ordeal by Roses) with Mishima; Kamaitachi; high-contrast darkroom manipulation.',
  ARRAY['Japanese Postwar Photography','Avant-Garde'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Masahisa Fukase', '深濑昌久', 1934, 2012, 'Japanese',
  'Photographer of The Ravens (Karasu), one of the most celebrated photobooks of the 20th century.',
  '35mm; The Ravens 1976–1982; personal and dark introspection; Solitude of Ravens.',
  ARRAY['Japanese Postwar Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Hiroshi Sugimoto', '杉本博司', 1948, NULL, 'Japanese',
  'Conceptual photographer exploring time, memory, and the architecture of seeing.',
  '8x10 view camera; Seascapes; Theaters; Dioramas; long exposures; Conceptual minimalism.',
  ARRAY['Conceptual Photography','Large Format'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 1980s–1990s 杜塞尔多夫学派 / 后现代 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Andreas Gursky', '安德烈亚斯·古尔斯基', 1955, NULL, 'German',
  'Large-format photographer of global capitalism; some of the highest auction prices in photography.',
  'Large format + digital manipulation; Rhine II; 99 Cent; monumental scale prints; Becher student.',
  ARRAY['Düsseldorf School','Large Format'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Thomas Struth', '托马斯·斯特鲁斯', 1954, NULL, 'German',
  'Photographer of city streets, museums, and family portraits with clinical precision.',
  'Large format; Unconscious Places; Museum Photographs; Becher student; precise frontal framing.',
  ARRAY['Düsseldorf School'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Candida Höfer', '坎迪达·霍弗', 1944, NULL, 'German',
  'Photographer of grand interior spaces—libraries, museums, and opera houses devoid of people.',
  'Large format 8x10 digital; Libraries series; symmetrical compositions; Becher student.',
  ARRAY['Düsseldorf School'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Bernd and Hilla Becher', '贝歇夫妇', 1931, 2007, 'German',
  'Husband-and-wife team whose typologies of industrial structures founded the Düsseldorf School.',
  '8x10 view camera; Water Towers; Blast Furnaces; deadpan typological method; taught Gursky/Struth/Höfer.',
  ARRAY['Düsseldorf School','Typology','Conceptual Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Cindy Sherman', '辛迪·舍曼', 1954, NULL, 'American',
  'Conceptual photographer whose staged self-portraits interrogate identity and representation.',
  'Untitled Film Stills 1977–1980; medium format; self-transformation; feminist critique.',
  ARRAY['Postmodern Photography','Staged Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Jeff Wall', '杰夫·沃尔', 1946, NULL, 'Canadian',
  'Large-scale staged photographs presented as backlit lightbox transparencies.',
  'Large format; lightbox presentation; Picture for Women; A Sudden Gust of Wind; cinematic staging.',
  ARRAY['Vancouver School','Staged Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Nan Goldin', '南·戈尔丁', 1953, NULL, 'American',
  'Photographer whose intimate diaristic work documents friends, lovers, and queer communities.',
  '35mm snapshot aesthetic; The Ballad of Sexual Dependency; slideshow format; raw intimacy.',
  ARRAY['Diary Photography','Snapshot Aesthetic'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 1990s–2020s 当代 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Wolfgang Tillmans', '沃尔夫冈·蒂尔曼斯', 1968, NULL, 'German',
  'First photographer to win the Turner Prize; diverse practice from clubs to abstraction.',
  '35mm and medium format; casual installation methods; Concorde; Freischwimmer.',
  ARRAY['Contemporary Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Alec Soth', '埃里克·索斯', 1969, NULL, 'American',
  'Member of Magnum; large-format road-trip photography along American rivers.',
  '8x10 view camera; Sleeping by the Mississippi; Niagara; Magnum member; poetic Americana.',
  ARRAY['Contemporary Photography','Large Format'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Taryn Simon', '塔林·西蒙', 1975, NULL, 'American',
  'Conceptual photographer using image and text to expose hidden systems of power.',
  'Medium format; An American Index of the Hidden and Unfamiliar; The Innocents; archival method.',
  ARRAY['Conceptual Photography','Contemporary Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

-- ═══ 中国摄影 ═══

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Lang Jingshan', '郎静山', 1892, 1995, 'Chinese',
  'Pioneer of Chinese photography known for "composite photography" blending Chinese painting aesthetics.',
  'Composite printing technique; jijing (artistic conception); landscapes in the literati tradition.',
  ARRAY['Chinese Pictorialism'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, biography, method_notes, movements, review_status)
VALUES('Lu Nan', '吕楠', 1962, NULL, 'Chinese',
  'Documentary photographer of marginalized communities; one of China''s most respected photojournalists.',
  '35mm; The Forgotten People (psychiatric patients); three-part trilogy over 15 years.',
  ARRAY['Chinese Documentary'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET biography = EXCLUDED.biography;

COMMIT;
