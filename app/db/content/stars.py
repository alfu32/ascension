"""The star catalogue: real stars at their real places (docs/database.md "Star catalogue").

Sources (primary; acknowledgements in docs/database.md):
- Positions and distances: the Hipparcos catalogue, new reduction (van Leeuwen 2007, A&A 474, 653;
  ESA; CDS catalogue I/311). Right ascension and declination are ICRS at epoch 1991.25 moved to
  J2000.0 with the catalogue's proper motions, in milli-degrees; the distance is 1 / parallax,
  in hundredths of a light-year (1 parsec = 3.261564 ly). A star needs a parallax at least three
  times its formal error; Deneb's is too uncertain, so its distance is the spectroscopic
  802 pc of Schiller and Przybilla (2008, A&A 479, 849).
- Proper names, Bayer-style names and constellations of the named stars: the IAU Catalog of Star
  Names (IAU Working Group on Star Names); other Bayer and Flamsteed designations: the Yale
  Bright Star Catalogue (Hoffleit and Warren 1991, CDS V/50), matched to Hipparcos by Henry Draper
  number or position. Constellation abbreviations and names are the IAU's.
- Stick figures: written for this project from the traditional figure of each constellation
  (which star joins which); they follow no existing line file.

The map position is computed here, once, into integers: the direction is exact and the distance
is compressed by `RADIAL_SCALE * sqrt(distance in light-years)`, so constellations keep their
true shapes seen from the Sun and the order of distances is kept. The galactic longitude (which
galactic quadrant a star lies in, task 89) is computed here too. The simulation and map
generator read integers only.

Curation (task 161, owner 2026-11-26): the map positions are curated so that no two star systems
lie on top of each other. Components of one physical multiple ("Alpha1 Centauri" and "Alpha2
Centauri") closer than `MIN_SPACING` are one system; every other star closer than that to a star
placed before it moves along its line of sight (only its map radius changes), or, failing that,
to the nearest free point. Right ascension, declination and distance stay the catalogue's; only
x, y, z move. `curate()` does it and `curation_summary()` reports it; the result is recorded
below `MIN_SPACING`.
"""
import math
import re

# Map units per sqrt(light-year): Proxima Centauri (4.2 ly) lies at about 205, Deneb (2600 ly) at 5100.
RADIAL_SCALE = 100

# The galactic frame (IAU 1958, in J2000 coordinates): right ascension and declination of the north
# galactic pole, and the galactic longitude of the north celestial pole (task 89).
GALACTIC_POLE_RA_DEG = 192.859
GALACTIC_POLE_DEC_DEG = 27.128
GALACTIC_NCP_LON_DEG = 122.932

# IAU abbreviation: (name, genitive)
CONSTELLATIONS = {
    'And': ('Andromeda', 'Andromedae'),
    'Ant': ('Antlia', 'Antliae'),
    'Aps': ('Apus', 'Apodis'),
    'Aql': ('Aquila', 'Aquilae'),
    'Aqr': ('Aquarius', 'Aquarii'),
    'Ara': ('Ara', 'Arae'),
    'Ari': ('Aries', 'Arietis'),
    'Aur': ('Auriga', 'Aurigae'),
    'Boo': ('Bootes', 'Bootis'),
    'CMa': ('Canis Major', 'Canis Majoris'),
    'CMi': ('Canis Minor', 'Canis Minoris'),
    'CVn': ('Canes Venatici', 'Canum Venaticorum'),
    'Cae': ('Caelum', 'Caeli'),
    'Cam': ('Camelopardalis', 'Camelopardalis'),
    'Cap': ('Capricornus', 'Capricorni'),
    'Car': ('Carina', 'Carinae'),
    'Cas': ('Cassiopeia', 'Cassiopeiae'),
    'Cen': ('Centaurus', 'Centauri'),
    'Cep': ('Cepheus', 'Cephei'),
    'Cet': ('Cetus', 'Ceti'),
    'Cha': ('Chamaeleon', 'Chamaeleontis'),
    'Cir': ('Circinus', 'Circini'),
    'Cnc': ('Cancer', 'Cancri'),
    'Col': ('Columba', 'Columbae'),
    'Com': ('Coma Berenices', 'Comae Berenices'),
    'CrA': ('Corona Australis', 'Coronae Australis'),
    'CrB': ('Corona Borealis', 'Coronae Borealis'),
    'Crt': ('Crater', 'Crateris'),
    'Cru': ('Crux', 'Crucis'),
    'Crv': ('Corvus', 'Corvi'),
    'Cyg': ('Cygnus', 'Cygni'),
    'Del': ('Delphinus', 'Delphini'),
    'Dor': ('Dorado', 'Doradus'),
    'Dra': ('Draco', 'Draconis'),
    'Equ': ('Equuleus', 'Equulei'),
    'Eri': ('Eridanus', 'Eridani'),
    'For': ('Fornax', 'Fornacis'),
    'Gem': ('Gemini', 'Geminorum'),
    'Gru': ('Grus', 'Gruis'),
    'Her': ('Hercules', 'Herculis'),
    'Hor': ('Horologium', 'Horologii'),
    'Hya': ('Hydra', 'Hydrae'),
    'Hyi': ('Hydrus', 'Hydri'),
    'Ind': ('Indus', 'Indi'),
    'LMi': ('Leo Minor', 'Leonis Minoris'),
    'Lac': ('Lacerta', 'Lacertae'),
    'Leo': ('Leo', 'Leonis'),
    'Lep': ('Lepus', 'Leporis'),
    'Lib': ('Libra', 'Librae'),
    'Lup': ('Lupus', 'Lupi'),
    'Lyn': ('Lynx', 'Lyncis'),
    'Lyr': ('Lyra', 'Lyrae'),
    'Mon': ('Monoceros', 'Monocerotis'),
    'Mus': ('Musca', 'Muscae'),
    'Nor': ('Norma', 'Normae'),
    'Oct': ('Octans', 'Octantis'),
    'Oph': ('Ophiuchus', 'Ophiuchi'),
    'Ori': ('Orion', 'Orionis'),
    'Pav': ('Pavo', 'Pavonis'),
    'Peg': ('Pegasus', 'Pegasi'),
    'Per': ('Perseus', 'Persei'),
    'Phe': ('Phoenix', 'Phoenicis'),
    'Pic': ('Pictor', 'Pictoris'),
    'PsA': ('Piscis Austrinus', 'Piscis Austrini'),
    'Psc': ('Pisces', 'Piscium'),
    'Pup': ('Puppis', 'Puppis'),
    'Pyx': ('Pyxis', 'Pyxidis'),
    'Ret': ('Reticulum', 'Reticuli'),
    'Scl': ('Sculptor', 'Sculptoris'),
    'Sco': ('Scorpius', 'Scorpii'),
    'Sct': ('Scutum', 'Scuti'),
    'Ser': ('Serpens', 'Serpentis'),
    'Sex': ('Sextans', 'Sextantis'),
    'Sge': ('Sagitta', 'Sagittae'),
    'Sgr': ('Sagittarius', 'Sagittarii'),
    'Tau': ('Taurus', 'Tauri'),
    'Tel': ('Telescopium', 'Telescopii'),
    'TrA': ('Triangulum Australe', 'Trianguli Australis'),
    'Tri': ('Triangulum', 'Trianguli'),
    'Tuc': ('Tucana', 'Tucanae'),
    'UMa': ('Ursa Major', 'Ursae Majoris'),
    'UMi': ('Ursa Minor', 'Ursae Minoris'),
    'Vel': ('Vela', 'Velorum'),
    'Vir': ('Virgo', 'Virginis'),
    'Vol': ('Volans', 'Volantis'),
    'Vul': ('Vulpecula', 'Vulpeculae'),
}

# Hipparcos number, name, designation, constellation, right ascension, declination (milli-degrees),
# distance (1/100 light-year)
STARS = [
    (677, 'Alpheratz', 'Alpha Andromedae', 'And', 2097, 29090, 9701),
    (746, 'Caph', 'Beta Cassiopeiae', 'Cas', 2295, 59150, 5474),
    (765, 'Epsilon Phoenicis', 'Epsilon Phoenicis', 'Phe', 2353, -45747, 14419),
    (1067, 'Algenib', 'Gamma Pegasi', 'Peg', 3309, 15184, 39154),
    (1562, 'Iota Ceti', 'Iota Ceti', 'Cet', 4857, -8824, 27454),
    (1599, 'Zeta Tucanae', 'Zeta Tucanae', 'Tuc', 5018, -64875, 2801),
    (2021, 'Beta Hydri', 'Beta Hydri', 'Hyi', 6438, -77254, 2433),
    (2072, 'Kappa Phoenicis', 'Kappa Phoenicis', 'Phe', 6551, -43680, 7766),
    (2081, 'Ankaa', 'Alpha Phoenicis', 'Phe', 6571, -42306, 8472),
    (2484, 'Beta1 Tucanae', 'Beta1 Tucanae', 'Tuc', 7886, -62958, 13505),
    (2599, 'Kappa Cassiopeiae', 'Kappa Cassiopeiae', 'Cas', 8250, 62932, 446790),
    (2912, 'Pi Andromedae', 'Pi Andromedae', 'And', 9220, 33719, 59845),
    (2920, 'Fulu', 'Zeta Cassiopeiae', 'Cas', 9243, 53897, 59301),
    (3031, 'Epsilon Andromedae', 'Epsilon Andromedae', 'And', 9639, 29312, 16382),
    (3092, 'Delta Andromedae', 'Delta Andromedae', 'And', 9832, 30861, 10552),
    (3179, 'Schedar', 'Alpha Cassiopeiae', 'Cas', 10127, 56537, 22824),
    (3405, 'Eta Phoenicis', 'Eta Phoenicis', 'Phe', 10838, -57463, 24634),
    (3419, 'Diphda', 'Beta Ceti', 'Cet', 10897, -17987, 9632),
    (3693, 'Zeta Andromedae', 'Zeta Andromedae', 'And', 11835, 24267, 18919),
    (3786, 'Delta Piscium', 'Delta Piscium', 'Psc', 12171, 7585, 31122),
    (3821, 'Achird', 'Eta Cassiopeiae', 'Cas', 12276, 57815, 1942),
    (4427, 'Gamma Cassiopeiae', 'Gamma Cassiopeiae', 'Cas', 14177, 60717, 54908),
    (4436, 'Mu Andromedae', 'Mu Andromedae', 'And', 14188, 38499, 12974),
    (4463, 'Eta Andromedae', 'Eta Andromedae', 'And', 14302, 23418, 22969),
    (4577, 'Alpha Sculptoris', 'Alpha Sculptoris', 'Scl', 14651, -29357, 77656),
    (4906, 'Epsilon Piscium', 'Epsilon Piscium', 'Psc', 15736, 7890, 18180),
    (5348, 'Wurren', 'Zeta Phoenicis', 'Phe', 17096, -55246, 29868),
    (5364, 'Eta Ceti', 'Eta Ceti', 'Cet', 17147, -10182, 12392),
    (5434, 'Phi Andromedae', 'Phi Andromedae', 'And', 17376, 47242, 71683),
    (5447, 'Mirach', 'Beta Andromedae', 'And', 17433, 35621, 19743),
    (5542, 'Theta Cassiopeiae', 'Theta Cassiopeiae', 'Cas', 17776, 55150, 13356),
    (6537, 'Theta Ceti', 'Theta Ceti', 'Cet', 21006, -8183, 11380),
    (6686, 'Ruchbah', 'Delta Cassiopeiae', 'Cas', 21454, 60235, 9941),
    (6867, 'Gamma Phoenicis', 'Gamma Phoenicis', 'Phe', 22091, -43318, 23364),
    (7083, 'Delta Phoenicis', 'Delta Phoenicis', 'Phe', 22813, -49073, 14212),
    (7097, 'Alpherg', 'Eta Piscium', 'Psc', 22871, 15346, 34958),
    (7513, 'Titawin', 'Upsilon Andromedae', 'And', 24199, 41405, 4400),
    (7588, 'Achernar', 'Alpha Eridani', 'Eri', 24429, -57237, 13944),
    (7607, 'Nembus', 'HR 464', 'And', 24498, 48628, 17716),
    (7884, 'Nu Piscium', 'Nu Piscium', 'Psc', 25358, 5488, 36320),
    (8068, 'Phi Persei', 'Phi Persei', 'Per', 25915, 50689, 71841),
    (8102, 'Tau Ceti', 'Tau Ceti', 'Cet', 26017, -15937, 1191),
    (8198, 'Torcular', 'Omicron Piscium', 'Psc', 26348, 9158, 27948),
    (8645, 'Baten Kaitos', 'Zeta Ceti', 'Cet', 27865, -10335, 23498),
    (8796, 'Mothallah', 'Alpha Trianguli', 'Tri', 28270, 29579, 6333),
    (8837, 'Psi Phoenicis', 'Psi Phoenicis', 'Phe', 28411, -46303, 34188),
    (8886, 'Segin', 'Epsilon Cassiopeiae', 'Cas', 28599, 63670, 41181),
    (8903, 'Sheratan', 'Beta Arietis', 'Ari', 28660, 20808, 5866),
    (9007, 'Chi Eridani', 'Chi Eridani', 'Eri', 28989, -51609, 5822),
    (9236, 'Alpha Hydri', 'Alpha Hydri', 'Hyi', 29692, -61570, 7179),
    (9347, 'Upsilon Ceti', 'Upsilon Ceti', 'Cet', 30001, -21078, 29278),
    (9487, 'Alrescha', 'Alpha Piscium', 'Psc', 30512, 2764, 15058),
    (9640, 'Almach', 'Gamma1 Andromedae', 'And', 30975, 42330, 39296),
    (9884, 'Hamal', 'Alpha Arietis', 'Ari', 31793, 23462, 6581),
    (10064, 'Beta Trianguli', 'Beta Trianguli', 'Tri', 32386, 34987, 12686),
    (10324, 'Xi1 Ceti', 'Xi1 Ceti', 'Cet', 33250, 8847, 38326),
    (10602, 'Phi Eridani', 'Phi Eridani', 'Eri', 34127, -51512, 15370),
    (10670, 'Gamma Trianguli', 'Gamma Trianguli', 'Tri', 34329, 33847, 11231),
    (10826, 'Omicron Ceti', 'Omicron Ceti', 'Cet', 34837, -2978, 29895),
    (11001, 'Delta Hydri', 'Delta Hydri', 'Hyi', 35437, -68659, 13968),
    (11407, 'Kappa Eridani', 'Kappa Eridani', 'Eri', 36746, -47704, 50803),
    (11484, 'Xi2 Ceti', 'Xi2 Ceti', 'Cet', 37040, 8460, 19311),
    (11767, 'Polaris', 'Alpha Ursae Minoris', 'UMi', 37955, 89264, 43257),
    (12387, 'Delta Ceti', 'Delta Ceti', 'Cet', 39871, 329, 64971),
    (12394, 'Epsilon Hydri', 'Epsilon Hydri', 'Hyi', 39897, -68267, 15184),
    (12486, 'Iota Eridani', 'Iota Eridani', 'Eri', 40167, -39855, 15065),
    (12706, 'Kaffaljidhma', 'Gamma Ceti', 'Cet', 40825, 3236, 7961),
    (12770, 'Pi Ceti', 'Pi Ceti', 'Cet', 41031, -13859, 39296),
    (12777, 'Theta Persei', 'Theta Persei', 'Per', 41050, 49228, 3629),
    (12828, 'Mu Ceti', 'Mu Ceti', 'Cet', 41236, 10114, 8406),
    (12843, 'Tau1 Eridani', 'Tau1 Eridani', 'Eri', 41276, -18573, 4638),
    (13147, 'Beta Fornacis', 'Beta Fornacis', 'For', 42273, -32406, 17266),
    (13209, 'Bharani', 'HR 838', 'Ari', 42496, 27261, 16565),
    (13268, 'Miram', 'Eta Persei', 'Per', 42674, 55895, 87913),
    (13531, 'Tau Persei', 'Tau Persei', 'Per', 43564, 52762, 25421),
    (13701, 'Azha', 'Eta Eridani', 'Eri', 44107, -8898, 13652),
    (13847, 'Acamar', 'Theta1 Eridani', 'Eri', 44565, -40305, 16122),
    (14135, 'Menkar', 'Alpha Ceti', 'Cet', 45570, 4090, 24916),
    (14146, 'Tau3 Eridani', 'Tau3 Eridani', 'Eri', 45598, -23624, 8863),
    (14328, 'Gamma Persei', 'Gamma Persei', 'Per', 46199, 53506, 24322),
    (14354, 'Rho Persei', 'Rho Persei', 'Per', 46294, 38840, 30769),
    (14576, 'Algol', 'Beta Persei', 'Per', 47042, 40956, 8992),
    (14632, 'Iota Persei', 'Iota Persei', 'Per', 47267, 49613, 3438),
    (14668, 'Misam', 'Kappa Persei', 'Per', 47374, 44858, 11274),
    (14838, 'Botein', 'Delta Arietis', 'Ari', 47907, 19727, 16970),
    (14879, 'Dalim', 'Alpha Fornacis', 'For', 48019, -28988, 4643),
    (15474, 'Tau4 Eridani', 'Tau4 Eridani', 'Eri', 49879, -21758, 30453),
    (15863, 'Mirfak', 'Alpha Persei', 'Per', 51081, 49861, 50645),
    (15900, 'Omicron Tauri', 'Omicron Tauri', 'Tau', 51203, 9029, 29095),
    (16083, 'Xi Tauri', 'Xi Tauri', 'Tau', 51792, 9733, 20907),
    (16335, 'Sigma Persei', 'Sigma Persei', 'Per', 52644, 47995, 35960),
    (16537, 'Epsilon Eridani', 'Epsilon Eridani', 'Eri', 53233, -9458, 1049),
    (16611, 'Tau5 Eridani', 'Tau5 Eridani', 'Eri', 53447, -21633, 29331),
    (16826, 'Psi Persei', 'Psi Persei', 'Per', 54122, 48193, 58346),
    (17358, 'Delta Persei', 'Delta Persei', 'Per', 55731, 47788, 51607),
    (17378, 'Rana', 'Delta Eridani', 'Eri', 55812, -9763, 2949),
    (17440, 'Beta Reticuli', 'Beta Reticuli', 'Ret', 56050, -64807, 9739),
    (17448, 'Atik', 'Omicron Persei', 'Per', 56080, 32288, 112081),
    (17499, 'Electra', 'HR 1142', 'Tau', 56219, 24113, 40466),
    (17529, 'Nu Persei', 'Nu Persei', 'Per', 56298, 42579, 55658),
    (17531, 'Taygeta', 'HR 1145', 'Tau', 56302, 24467, 40923),
    (17573, 'Maia', 'HR 1149', 'Tau', 56457, 24368, 38326),
    (17593, 'Pi Eridani', 'Pi Eridani', 'Eri', 56536, -12102, 48106),
    (17608, 'Merope', 'HR 1156', 'Tau', 56582, 23948, 38014),
    (17651, 'Tau6 Eridani', 'Tau6 Eridani', 'Eri', 56712, -23250, 5749),
    (17678, 'Gamma Hydri', 'Gamma Hydri', 'Hyi', 56810, -74239, 21401),
    (17702, 'Alcyone', 'Eta Tauri', 'Tau', 56871, 24105, 40316),
    (17847, 'Atlas', 'HR 1178', 'Tau', 57291, 24053, 38236),
    (18246, 'Zeta Persei', 'Zeta Persei', 'Per', 58533, 31884, 75151),
    (18532, 'Epsilon Persei', 'Epsilon Persei', 'Per', 59463, 40010, 63827),
    (18543, 'Zaurak', 'Gamma Eridani', 'Eri', 59507, -13509, 20334),
    (18614, 'Menkib', 'Xi Persei', 'Per', 59741, 35791, 124487),
    (18724, 'Lambda Tauri', 'Lambda Tauri', 'Tau', 60170, 12490, 48391),
    (18907, 'Nu Tauri', 'Nu Tauri', 'Tau', 60789, 5989, 11694),
    (19167, 'Lambda Persei', 'Lambda Persei', 'Per', 61646, 50351, 42194),
    (19587, 'Beid', 'Omicron1 Eridani', 'Eri', 62966, -6838, 12170),
    (19747, 'Alpha Horologii', 'Alpha Horologii', 'Hor', 63500, -42294, 11501),
    (19780, 'Alpha Reticuli', 'Alpha Reticuli', 'Ret', 63606, -62474, 16162),
    (19812, 'Mu Persei', 'Mu Persei', 'Per', 63724, 48409, 90098),
    (19849, 'Keid', 'Omicron2 Eridani', 'Eri', 63818, -7653, 1626),
    (19860, 'Mu Tauri', 'Mu Tauri', 'Tau', 63884, 8892, 45553),
    (19893, 'Gamma Doradus', 'Gamma Doradus', 'Dor', 64007, -51487, 6674),
    (19921, 'Epsilon Reticuli', 'Epsilon Reticuli', 'Ret', 64121, -59302, 5949),
    (20042, 'Upsilon4 Eridani', 'Upsilon4 Eridani', 'Eri', 64474, -33798, 17794),
    (20205, 'Prima Hyadum', 'Gamma Tauri', 'Tau', 64948, 15628, 16154),
    (20455, 'Secunda Hyadum', 'Delta1 Tauri', 'Tau', 65734, 17543, 15561),
    (20535, 'Beemim', 'HR 1393', 'Eri', 66009, -34017, 29624),
    (20635, 'Kappa1 Tauri', 'Kappa1 Tauri', 'Tau', 66342, 22294, 15392),
    (20648, 'Delta3 Tauri', 'Delta3 Tauri', 'Tau', 66372, 17928, 14852),
    (20711, 'Upsilon Tauri', 'Upsilon Tauri', 'Tau', 66577, 22814, 15377),
    (20885, 'Theta1 Tauri', 'Theta1 Tauri', 'Tau', 67144, 15962, 15436),
    (20889, 'Ain', 'Epsilon Tauri', 'Tau', 67154, 19180, 14665),
    (20894, 'Chamukuy', 'Theta2 Tauri', 'Tau', 67166, 15871, 15037),
    (21281, 'Alpha Doradus', 'Alpha Doradus', 'Dor', 68499, -55045, 16864),
    (21393, 'Theemin', 'Upsilon2 Eridani', 'Eri', 68888, -30562, 21387),
    (21421, 'Aldebaran', 'Alpha Tauri', 'Tau', 68980, 16509, 6664),
    (21444, 'Nu Eridani', 'Nu Eridani', 'Eri', 69080, -3352, 67527),
    (21594, 'Sceptrum', 'HR 1481', 'Eri', 69545, -14304, 10985),
    (21770, 'Alpha Caeli', 'Alpha Caeli', 'Cae', 70140, -41864, 6577),
    (21881, 'Tau Tauri', 'Tau Tauri', 'Tau', 70561, 22957, 39824),
    (22109, 'Mu Eridani', 'Mu Eridani', 'Eri', 71376, -3255, 52185),
    (22449, 'Tabit', 'Pi3 Orionis', 'Ori', 72460, 6961, 2632),
    (22509, 'Pi2 Orionis', 'Pi2 Orionis', 'Ori', 72653, 8900, 22447),
    (22549, 'Pi4 Orionis', 'Pi4 Orionis', 'Ori', 72802, 5605, 105212),
    (22701, 'Omega Eridani', 'Omega Eridani', 'Eri', 73224, -5453, 23498),
    (22797, 'Pi5 Orionis', 'Pi5 Orionis', 'Ori', 73563, 2441, 134221),
    (22957, 'Omicron2 Orionis', 'Omicron2 Orionis', 'Ori', 74093, 13514, 18595),
    (23015, 'Hassaleh', 'Iota Aurigae', 'Aur', 74248, 33166, 49343),
    (23123, 'Pi6 Orionis', 'Pi6 Orionis', 'Ori', 74637, 1714, 94538),
    (23453, 'Saclateni', 'Zeta Aurigae', 'Aur', 75620, 41076, 78592),
    (23522, 'Beta Camelopardalis', 'Beta Camelopardalis', 'Cam', 75855, 60442, 87208),
    (23685, 'Epsilon Leporis', 'Epsilon Leporis', 'Lep', 76365, -22371, 21331),
    (23767, 'Haedus', 'Eta Aurigae', 'Aur', 76629, 41234, 24340),
    (23875, 'Cursa', 'Beta Eridani', 'Eri', 76962, -5086, 8936),
    (23972, 'Lambda Eridani', 'Lambda Eridani', 'Eri', 77287, -8754, 81133),
    (24244, 'Iota Leporis', 'Iota Leporis', 'Lep', 78075, -11869, 23181),
    (24305, 'Mu Leporis', 'Mu Leporis', 'Lep', 78233, -16205, 18595),
    (24327, 'Kappa Leporis', 'Kappa Leporis', 'Lep', 78308, -12941, 72803),
    (24331, 'Rho Orionis', 'Rho Orionis', 'Ori', 78323, 2861, 34995),
    (24436, 'Rigel', 'Beta Orionis', 'Ori', 78634, -8202, 86285),
    (24608, 'Capella', 'Alpha Aurigae', 'Aur', 79172, 45998, 4280),
    (24674, 'Tau Orionis', 'Tau Orionis', 'Ori', 79402, -6844, 49418),
    (24845, 'Lambda Leporis', 'Lambda Leporis', 'Lep', 79894, -13177, 85158),
    (25281, 'Eta Orionis', 'Eta Orionis', 'Ori', 81119, -2397, 97652),
    (25336, 'Bellatrix', 'Gamma Orionis', 'Ori', 81283, 6350, 25244),
    (25428, 'Elnath', 'Beta Tauri', 'Tau', 81573, 28607, 13389),
    (25606, 'Nihal', 'Beta Leporis', 'Lep', 82061, -20759, 16035),
    (25859, 'Epsilon Columbae', 'Epsilon Columbae', 'Col', 82803, -35471, 26324),
    (25930, 'Mintaka', 'Delta Orionis', 'Ori', 83002, -299, 69248),
    (25985, 'Arneb', 'Alpha Leporis', 'Lep', 83183, -17822, 221875),
    (26069, 'Beta Doradus', 'Beta Doradus', 'Dor', 83406, -62490, 100666),
    (26176, 'Phi1 Orionis', 'Phi1 Orionis', 'Ori', 83705, 9490, 108719),
    (26207, 'Meissa', 'Lambda Orionis', 'Ori', 83784, 9934, 109817),
    (26241, 'Hatysa', 'Iota Orionis', 'Ori', 83858, -5910, 232969),
    (26311, 'Alnilam', 'Epsilon Orionis', 'Ori', 84053, -1202, 197671),
    (26366, 'Phi2 Orionis', 'Phi2 Orionis', 'Ori', 84227, 9291, 11749),
    (26451, 'Tianguan', 'Zeta Tauri', 'Tau', 84411, 21143, 44496),
    (26634, 'Phact', 'Alpha Columbae', 'Col', 84912, -34074, 26134),
    (26727, 'Alnitak', 'Zeta Orionis', 'Ori', 85190, -1943, 73624),
    (27072, 'Gamma Leporis', 'Gamma Leporis', 'Lep', 86116, -22448, 2912),
    (27100, 'Delta Doradus', 'Delta Doradus', 'Dor', 86193, -65736, 14961),
    (27288, 'Zeta Leporis', 'Zeta Leporis', 'Lep', 86739, -14822, 7047),
    (27321, 'Beta Pictoris', 'Beta Pictoris', 'Pic', 86821, -51067, 6341),
    (27366, 'Saiph', 'Kappa Orionis', 'Ori', 86939, -9670, 64714),
    (27628, 'Wazn', 'Beta Columbae', 'Col', 87740, -35768, 8718),
    (27654, 'Delta Leporis', 'Delta Leporis', 'Lep', 87830, -20879, 11372),
    (27673, 'Nu Aurigae', 'Nu Aurigae', 'Aur', 87873, 39148, 23034),
    (27913, 'Chi1 Orionis', 'Chi1 Orionis', 'Ori', 88596, 20276, 2826),
    (27989, 'Betelgeuse', 'Alpha Orionis', 'Ori', 88793, 7407, 49795),
    (28103, 'Eta Leporis', 'Eta Leporis', 'Lep', 89101, -14168, 4853),
    (28199, 'Gamma Columbae', 'Gamma Columbae', 'Col', 89384, -35283, 86975),
    (28328, 'Eta Columbae', 'Eta Columbae', 'Col', 89787, -42815, 47201),
    (28358, 'Delta Aurigae', 'Delta Aurigae', 'Aur', 89882, 54285, 12603),
    (28360, 'Menkalinan', 'Beta Aurigae', 'Aur', 89882, 44947, 8111),
    (28380, 'Mahasim', 'Theta Aurigae', 'Aur', 89930, 37213, 16556),
    (28404, 'Pi Aurigae', 'Pi Aurigae', 'Aur', 89984, 45937, 75151),
    (28614, 'Mu Orionis', 'Mu Orionis', 'Ori', 90596, 9647, 15494),
    (29038, 'Nu Orionis', 'Nu Orionis', 'Ori', 91893, 14768, 51607),
    (29426, 'Xi Orionis', 'Xi Orionis', 'Ori', 92985, 14209, 60737),
    (29651, 'Gamma Monocerotis', 'Gamma Monocerotis', 'Mon', 93714, -6275, 49795),
    (29655, 'Propus', 'Eta Geminorum', 'Gem', 93719, 22507, 38462),
    (29696, 'Kappa Aurigae', 'Kappa Aurigae', 'Aur', 93845, 29498, 17697),
    (29807, 'Kappa Columbae', 'Kappa Columbae', 'Col', 94138, -35141, 18252),
    (30122, 'Furud', 'Zeta Canis Majoris', 'CMa', 95078, -30063, 36240),
    (30277, 'Delta Columbae', 'Delta Columbae', 'Col', 95528, -33436, 23397),
    (30324, 'Mirzam', 'Beta Canis Majoris', 'CMa', 95675, -17956, 49268),
    (30343, 'Tejat', 'Mu Geminorum', 'Gem', 95740, 22514, 23165),
    (30419, 'Epsilon Monocerotis', 'Epsilon Monocerotis', 'Mon', 95942, 4593, 12229),
    (30438, 'Canopus', 'Alpha Carinae', 'Car', 95988, -52696, 30915),
    (30788, 'Lambda Canis Majoris', 'Lambda Canis Majoris', 'CMa', 97043, -32580, 42358),
    (30883, 'Nu Geminorum', 'Nu Geminorum', 'Gem', 97241, 20212, 54450),
    (31125, 'Xi1 Canis Majoris', 'Xi1 Canis Majoris', 'CMa', 97964, -23418, 138202),
    (31592, 'Nu2 Canis Majoris', 'Nu2 Canis Majoris', 'CMa', 99171, -19256, 6442),
    (31681, 'Alhena', 'Gamma Geminorum', 'Gem', 99428, 16399, 10930),
    (31685, 'Nu Puppis', 'Nu Puppis', 'Pup', 99440, -43196, 37148),
    (31700, 'Nu3 Canis Majoris', 'Nu3 Canis Majoris', 'CMa', 99473, -18237, 42139),
    (32246, 'Mebsuta', 'Epsilon Geminorum', 'Gem', 100983, 25131, 84496),
    (32349, 'Sirius', 'Alpha Canis Majoris', 'CMa', 101287, -16716, 860),
    (32362, 'Alzirr', 'Xi Geminorum', 'Gem', 101322, 12896, 5870),
    (32607, 'Alpha Pictoris', 'Alpha Pictoris', 'Pic', 102048, -61941, 9655),
    (32759, 'Kappa Canis Majoris', 'Kappa Canis Majoris', 'CMa', 102460, -32508, 65890),
    (32768, 'Tau Puppis', 'Tau Puppis', 'Pup', 102484, -50615, 18201),
    (33018, 'Theta Geminorum', 'Theta Geminorum', 'Gem', 103197, 33961, 18908),
    (33160, 'Theta Canis Majoris', 'Theta Canis Majoris', 'CMa', 103547, -12039, 26072),
    (33347, 'Iota Canis Majoris', 'Iota Canis Majoris', 'CMa', 104034, -17054, 250890),
    (33579, 'Adhara', 'Epsilon Canis Majoris', 'CMa', 104656, -28972, 40516),
    (33856, 'Nganurganity', 'Sigma Canis Majoris', 'CMa', 105430, -27935, 112081),
    (34045, 'Muliphein', 'Gamma Canis Majoris', 'CMa', 105940, -15633, 44195),
    (34088, 'Mekbuda', 'Zeta Geminorum', 'Gem', 106027, 20570, 137619),
    (34444, 'Wezen', 'Delta Canis Majoris', 'CMa', 107098, -26393, 160668),
    (34481, 'Gamma2 Volantis', 'Gamma2 Volantis', 'Vol', 107187, -70499, 13980),
    (34693, 'Tau Geminorum', 'Tau Geminorum', 'Gem', 107785, 30245, 32102),
    (34769, 'Delta Monocerotis', 'Delta Monocerotis', 'Mon', 107966, -493, 38417),
    (35037, 'Omega Canis Majoris', 'Omega Canis Majoris', 'CMa', 108703, -26773, 91105),
    (35228, 'Delta Volantis', 'Delta Volantis', 'Vol', 109208, -67957, 73791),
    (35264, 'Pi Puppis', 'Pi Puppis', 'Pup', 109286, -37097, 80732),
    (35350, 'Lambda Geminorum', 'Lambda Geminorum', 'Gem', 109523, 16540, 10088),
    (35550, 'Wasat', 'Delta Geminorum', 'Gem', 110031, 21982, 6047),
    (35904, 'Aludra', 'Eta Canis Majoris', 'CMa', 111024, -29303, 198876),
    (36046, 'Iota Geminorum', 'Iota Geminorum', 'Gem', 111432, 27798, 12035),
    (36188, 'Gomeisa', 'Beta Canis Minoris', 'CMi', 111788, 8289, 16170),
    (36284, 'Gamma Canis Minoris', 'Gamma Canis Minoris', 'CMi', 112041, 8926, 31820),
    (36366, 'Rho Geminorum', 'Rho Geminorum', 'Gem', 112278, 31785, 5886),
    (36377, 'Sigma Puppis', 'Sigma Puppis', 'Pup', 112308, -43301, 19368),
    (36850, 'Castor', 'Alpha Geminorum', 'Gem', 113649, 31888, 5087),
    (36962, 'Upsilon Geminorum', 'Upsilon Geminorum', 'Gem', 113981, 26896, 27089),
    (37279, 'Procyon', 'Alpha Canis Minoris', 'CMi', 114825, 5225, 1146),
    (37447, 'Alpha Monocerotis', 'Alpha Monocerotis', 'Mon', 115312, -9551, 14778),
    (37504, 'Zeta Volantis', 'Zeta Volantis', 'Vol', 115455, -72606, 14101),
    (37629, 'Sigma Geminorum', 'Sigma Geminorum', 'Gem', 115828, 28884, 12506),
    (37740, 'Kappa Geminorum', 'Kappa Geminorum', 'Gem', 116112, 24398, 14138),
    (37826, 'Pollux', 'Beta Geminorum', 'Gem', 116329, 28026, 3378),
    (38070, 'Omicron Puppis', 'Omicron Puppis', 'Pup', 117022, -25937, 141807),
    (38170, 'Azmidi', 'Xi Puppis', 'Pup', 117324, -24860, 119910),
    (38827, 'Chi Carinae', 'Chi Carinae', 'Car', 119195, -52982, 45489),
    (39429, 'Naos', 'Zeta Puppis', 'Pup', 120896, -40003, 108358),
    (39757, 'Tureis', 'Rho Puppis', 'Pup', 121886, -24304, 6354),
    (39794, 'Epsilon Volantis', 'Epsilon Volantis', 'Vol', 121982, -68617, 56234),
    (39863, 'Zeta Monocerotis', 'Zeta Monocerotis', 'Mon', 122149, -2984, 105895),
    (39953, 'Gamma2 Velorum', 'Gamma2 Velorum', 'Vel', 122383, -47337, 111697),
    (40526, 'Tarf', 'Beta Cancri', 'Cnc', 124129, 9186, 30340),
    (40702, 'Alpha Chamaeleontis', 'Alpha Chamaeleontis', 'Cha', 124631, -76920, 6380),
    (40888, 'Theta Chamaeleontis', 'Theta Chamaeleontis', 'Cha', 125161, -77484, 15531),
    (41037, 'Avior', 'Epsilon Carinae', 'Car', 125628, -59509, 60511),
    (41075, 'Alsciaukat', 'HR 3275', 'Lyn', 125709, 43188, 38236),
    (41312, 'Beta Volantis', 'Beta Volantis', 'Vol', 126434, -66137, 10754),
    (41704, 'Muscida', 'Omicron Ursae Majoris', 'UMa', 127566, 60718, 17911),
    (42313, 'Delta Hydrae', 'Delta Hydrae', 'Hya', 129414, 5704, 16035),
    (42402, 'Minchir', 'Sigma Hydrae', 'Hya', 129689, 3341, 37275),
    (42515, 'Beta Pyxidis', 'Beta Pyxidis', 'Pyx', 130026, -35308, 41602),
    (42799, 'Eta Hydrae', 'Eta Hydrae', 'Hya', 130806, 3399, 58661),
    (42828, 'Alpha Pyxidis', 'Alpha Pyxidis', 'Pyx', 130898, -33186, 87913),
    (42911, 'Asellus Australis', 'Delta Cancri', 'Cnc', 131171, 18154, 13057),
    (42913, 'Alsephina', 'Delta Velorum', 'Vel', 131176, -54709, 8055),
    (43103, 'Iota Cancri', 'Iota Cancri', 'Cnc', 131674, 28760, 33112),
    (43109, 'Ashlesha', 'Epsilon Hydrae', 'Hya', 131694, 6419, 12927),
    (43234, 'Rho Hydrae', 'Rho Hydrae', 'Hya', 132108, 5838, 35413),
    (43409, 'Gamma Pyxidis', 'Gamma Pyxidis', 'Pyx', 132633, -27710, 20735),
    (43813, 'Zeta Hydrae', 'Zeta Hydrae', 'Hya', 133848, 5946, 16717),
    (44066, 'Acubens', 'Alpha Cancri', 'Cnc', 134622, 11858, 18831),
    (44127, 'Talitha', 'Iota Ursae Majoris', 'UMa', 134802, 48042, 4732),
    (44382, 'Alpha Volantis', 'Alpha Volantis', 'Vol', 135612, -66396, 12492),
    (44471, 'Alkaphrah', 'Kappa Ursae Majoris', 'UMa', 135906, 47157, 35841),
    (44816, 'Suhail', 'Lambda Velorum', 'Vel', 136999, -43433, 54450),
    (45238, 'Miaplacidus', 'Beta Carinae', 'Car', 138300, -69717, 11317),
    (45336, 'Theta Hydrae', 'Theta Hydrae', 'Hya', 138591, 2314, 11349),
    (45556, 'Aspidiske', 'Iota Carinae', 'Car', 139273, -59275, 76563),
    (45860, 'Alpha Lyncis', 'Alpha Lyncis', 'Lyn', 140264, 34393, 20309),
    (45941, 'Markeb', 'Kappa Velorum', 'Vel', 140528, -55011, 57220),
    (46146, 'Kappa Leonis', 'Kappa Leonis', 'Leo', 141164, 26182, 20133),
    (46390, 'Alphard', 'Alpha Hydrae', 'Hya', 141897, -8659, 18030),
    (46651, 'Psi Velorum', 'Psi Velorum', 'Vel', 142675, -40467, 6137),
    (46750, 'Alterf', 'Lambda Leonis', 'Leo', 142930, 22968, 32912),
    (46853, 'Theta Ursae Majoris', 'Theta Ursae Majoris', 'UMa', 143214, 51677, 4396),
    (47431, 'Ukdah', 'Iota Hydrae', 'Hya', 144964, -1143, 26324),
    (47508, 'Subra', 'Omicron Leonis', 'Leo', 145288, 9892, 13031),
    (47908, 'Epsilon Leonis', 'Epsilon Leonis', 'Leo', 146463, 23774, 24671),
    (48002, 'Upsilon Carinae', 'Upsilon Carinae', 'Car', 146776, -65072, 143681),
    (48319, 'Upsilon Ursae Majoris', 'Upsilon Ursae Majoris', 'UMa', 147747, 59039, 11624),
    (48356, 'Zhang', 'Upsilon1 Hydrae', 'Hya', 147870, -14847, 26388),
    (48455, 'Rasalas', 'Mu Leonis', 'Leo', 148191, 26007, 12411),
    (48774, 'Phi Velorum', 'Phi Velorum', 'Vel', 149216, -54568, 159101),
    (49583, 'Eta Leonis', 'Eta Leonis', 'Leo', 151833, 16763, 126909),
    (49641, 'Alpha Sextantis', 'Alpha Sextantis', 'Sex', 151985, -372, 28337),
    (49669, 'Regulus', 'Alpha Leonis', 'Leo', 152093, 11967, 7930),
    (49841, 'Lambda Hydrae', 'Lambda Hydrae', 'Hya', 152647, -12354, 11255),
    (50099, 'Omega Carinae', 'Omega Carinae', 'Car', 153434, -70038, 34188),
    (50335, 'Adhafera', 'Zeta Leonis', 'Leo', 154173, 23417, 27408),
    (50372, 'Tania Borealis', 'Lambda Ursae Majoris', 'UMa', 154274, 42914, 13750),
    (50583, 'Algieba', 'Gamma1 Leonis', 'Leo', 154993, 19841, 13010),
    (50801, 'Tania Australis', 'Mu Ursae Majoris', 'UMa', 155582, 41500, 23034),
    (51069, 'Mu Hydrae', 'Mu Hydrae', 'Hya', 156523, -16836, 23414),
    (51172, 'Alpha Antliae', 'Alpha Antliae', 'Ant', 156788, -31068, 36606),
    (51233, 'Beta Leonis Minoris', 'Beta Leonis Minoris', 'LMi', 156971, 36707, 15392),
    (51624, 'Rho Leonis', 'Rho Leonis', 'Leo', 158203, 9307, 543594),
    (51839, 'Gamma Chamaeleontis', 'Gamma Chamaeleontis', 'Cha', 158867, -78608, 41761),
    (52419, 'Theta Carinae', 'Theta Carinae', 'Car', 160739, -64394, 45553),
    (52633, 'Delta2 Chamaeleontis', 'Delta2 Chamaeleontis', 'Cha', 161446, -80540, 35071),
    (52727, 'Mu Velorum', 'Mu Velorum', 'Vel', 161692, -49420, 11715),
    (52943, 'Nu Hydrae', 'Nu Hydrae', 'Hya', 162406, -16194, 14374),
    (53229, 'Praecipua', 'HR 4247', 'LMi', 163328, 34215, 9487),
    (53740, 'Alkes', 'Alpha Crateris', 'Crt', 164944, -18299, 15918),
    (53910, 'Merak', 'Beta Ursae Majoris', 'UMa', 165460, 56382, 7974),
    (54061, 'Dubhe', 'Alpha Ursae Majoris', 'UMa', 165932, 61751, 12289),
    (54539, 'Psi Ursae Majoris', 'Psi Ursae Majoris', 'UMa', 167416, 44498, 14451),
    (54682, 'Beta Crateris', 'Beta Crateris', 'Crt', 167915, -22826, 34010),
    (54872, 'Zosma', 'Delta Leonis', 'Leo', 168527, 20524, 5843),
    (54879, 'Chertan', 'Theta Leonis', 'Leo', 168560, 15430, 16506),
    (55084, 'Phi Leonis', 'Phi Leonis', 'Leo', 169165, -3652, 18417),
    (55219, 'Alula Borealis', 'Nu Ursae Majoris', 'UMa', 169620, 33094, 39921),
    (55282, 'Delta Crateris', 'Delta Crateris', 'Crt', 169835, -14779, 18574),
    (55425, 'Pi Centauri', 'Pi Centauri', 'Cen', 170252, -54491, 35763),
    (55434, 'Sigma Leonis', 'Sigma Leonis', 'Leo', 170284, 6029, 22008),
    (55642, 'Iota Leonis', 'Iota Leonis', 'Leo', 170981, 10530, 7722),
    (55705, 'Gamma Crateris', 'Gamma Crateris', 'Crt', 171221, -17684, 8232),
    (56211, 'Giausar', 'Lambda Draconis', 'Dra', 172851, 69331, 33315),
    (56343, 'Xi Hydrae', 'Xi Hydrae', 'Hya', 173250, -31858, 12963),
    (56561, 'Lambda Centauri', 'Lambda Centauri', 'Cen', 173945, -63020, 41976),
    (56647, 'Upsilon Leonis', 'Upsilon Leonis', 'Leo', 174237, -824, 18150),
    (57363, 'Lambda Muscae', 'Lambda Muscae', 'Mus', 176402, -66729, 12716),
    (57380, 'Nu Virginis', 'Nu Virginis', 'Vir', 176465, 6529, 29383),
    (57399, 'Taiyangshou', 'Chi Ursae Majoris', 'UMa', 176513, 47779, 18365),
    (57632, 'Denebola', 'Beta Leonis', 'Leo', 177265, 14572, 3588),
    (57757, 'Zavijava', 'Beta Virginis', 'Vir', 177674, 1765, 3565),
    (57936, 'Beta Hydrae', 'Beta Hydrae', 'Hya', 178227, -33908, 30974),
    (58001, 'Phecda', 'Gamma Ursae Majoris', 'UMa', 178458, 53695, 8318),
    (58758, 'Theta1 Crucis', 'Theta1 Crucis', 'Cru', 180756, -63313, 23498),
    (58948, 'Omicron Virginis', 'Omicron Virginis', 'Vir', 181302, 8733, 16324),
    (59072, 'Eta Crucis', 'Eta Crucis', 'Cru', 181720, -64614, 6443),
    (59196, 'Delta Centauri', 'Delta Centauri', 'Cen', 182090, -50722, 41496),
    (59199, 'Alchiba', 'Alpha Corvi', 'Crv', 182103, -24729, 4872),
    (59316, 'Epsilon Corvi', 'Epsilon Corvi', 'Crv', 182531, -22620, 31789),
    (59449, 'Rho Centauri', 'Rho Centauri', 'Cen', 182913, -52368, 37881),
    (59747, 'Imai', 'Delta Crucis', 'Cru', 183786, -58749, 34514),
    (59774, 'Megrez', 'Delta Ursae Majoris', 'UMa', 183857, 57033, 8051),
    (59803, 'Gienah', 'Gamma Corvi', 'Crv', 183952, -17542, 15363),
    (59929, 'Epsilon Muscae', 'Epsilon Muscae', 'Mus', 184393, -67961, 30144),
    (60000, 'Beta Chamaeleontis', 'Beta Chamaeleontis', 'Cha', 184587, -79312, 29840),
    (60009, 'Zeta Crucis', 'Zeta Crucis', 'Cru', 184609, -64003, 35763),
    (60129, 'Eta Virginis', 'Eta Virginis', 'Vir', 184976, -667, 26538),
    (60260, 'Ginan', 'Epsilon Crucis', 'Cru', 185340, -60401, 22985),
    (60718, 'Acrux', 'Alpha1 Crucis', 'Cru', 186650, -63099, 32197),
    (60742, 'Gamma Comae Berenices', 'Gamma Comae Berenices', 'Com', 186734, 28268, 16726),
    (60823, 'Sigma Centauri', 'Sigma Centauri', 'Cen', 187010, -50231, 41181),
    (60965, 'Algorab', 'Delta Corvi', 'Crv', 187466, -16515, 8686),
    (61084, 'Gacrux', 'Gamma Crucis', 'Cru', 187791, -57113, 8856),
    (61174, 'Eta Corvi', 'Eta Corvi', 'Crv', 188018, -16196, 5963),
    (61199, 'Gamma Muscae', 'Gamma Muscae', 'Mus', 188117, -72133, 32486),
    (61281, 'Kappa Draconis', 'Kappa Draconis', 'Dra', 188371, 69788, 49046),
    (61317, 'Chara', 'Beta Canum Venaticorum', 'CVn', 188436, 41357, 2753),
    (61359, 'Kraz', 'Beta Corvi', 'Crv', 188597, -23397, 14567),
    (61585, 'Alpha Muscae', 'Alpha Muscae', 'Mus', 189296, -69136, 31543),
    (61622, 'Tau Centauri', 'Tau Centauri', 'Cen', 189426, -48541, 13125),
    (61932, 'Gamma Centauri', 'Gamma Centauri', 'Cen', 190379, -48960, 13015),
    (61941, 'Porrima', 'Gamma Virginis', 'Vir', 190415, -1449, 3811),
    (62322, 'Beta Muscae', 'Beta Muscae', 'Mus', 191570, -68108, 34153),
    (62434, 'Mimosa', 'Beta Crucis', 'Cru', 191930, -59689, 27853),
    (62956, 'Alioth', 'Epsilon Ursae Majoris', 'UMa', 193507, 55960, 8255),
    (63003, 'Mu1 Crucis', 'Mu1 Crucis', 'Cru', 193648, -57178, 41443),
    (63090, 'Minelauva', 'Delta Virginis', 'Vir', 193901, 3397, 19839),
    (63125, 'Cor Caroli', 'Alpha2 Canum Venaticorum', 'CVn', 194007, 38318, 11480),
    (63608, 'Vindemiatrix', 'Epsilon Virginis', 'Vir', 195544, 10959, 10960),
    (63613, 'Delta Muscae', 'Delta Muscae', 'Mus', 195568, -71549, 9090),
    (64004, 'Xi2 Centauri', 'Xi2 Centauri', 'Cen', 196728, -49906, 46727),
    (64238, 'Theta Virginis', 'Theta Virginis', 'Vir', 197487, -5539, 31574),
    (64394, 'Beta Comae Berenices', 'Beta Comae Berenices', 'Com', 197968, 27878, 2978),
    (64962, 'Gamma Hydrae', 'Gamma Hydrae', 'Hya', 199730, -23172, 13384),
    (65109, 'Iota Centauri', 'Iota Centauri', 'Cen', 200149, -36712, 5878),
    (65378, 'Mizar', 'Zeta Ursae Majoris', 'UMa', 200981, 54925, 8581),
    (65474, 'Spica', 'Alpha Virginis', 'Vir', 201298, -11161, 24974),
    (65477, 'Alcor', 'HR 5062', 'UMa', 201306, 54988, 8172),
    (66249, 'Heze', 'Zeta Virginis', 'Vir', 203673, -596, 7408),
    (66657, 'Epsilon Centauri', 'Epsilon Centauri', 'Cen', 204972, -53466, 42747),
    (67275, 'Tau Bootis', 'Tau Bootis', 'Boo', 206816, 17457, 5094),
    (67301, 'Alkaid', 'Eta Ursae Majoris', 'UMa', 206885, 49313, 10394),
    (67459, 'Upsilon Bootis', 'Upsilon Bootis', 'Boo', 207369, 15798, 26345),
    (67464, 'Nu Centauri', 'Nu Centauri', 'Cen', 207376, -41688, 43662),
    (67472, 'Mu Centauri', 'Mu Centauri', 'Cen', 207404, -42474, 50567),
    (67927, 'Muphrid', 'Eta Bootis', 'Boo', 208671, 18398, 3717),
    (68002, 'Zeta Centauri', 'Zeta Centauri', 'Cen', 208885, -47288, 38192),
    (68245, 'Phi Centauri', 'Phi Centauri', 'Cen', 209568, -42101, 52521),
    (68282, 'Upsilon1 Centauri', 'Upsilon1 Centauri', 'Cen', 209670, -44804, 42747),
    (68520, 'Tau Virginis', 'Tau Virginis', 'Vir', 210412, 1545, 22494),
    (68702, 'Hadar', 'Beta Centauri', 'Cen', 210956, -60373, 39201),
    (68756, 'Thuban', 'Alpha Draconis', 'Dra', 211097, 64376, 30312),
    (68862, 'Chi Centauri', 'Chi Centauri', 'Cen', 211512, -41180, 51042),
    (68895, 'Pi Hydrae', 'Pi Hydrae', 'Hya', 211593, -26682, 10098),
    (68933, 'Menkent', 'Theta Centauri', 'Cen', 211671, -36370, 5882),
    (69427, 'Kang', 'Kappa Virginis', 'Vir', 213224, -10274, 25481),
    (69673, 'Arcturus', 'Alpha Bootis', 'Boo', 213915, 19182, 3672),
    (69701, 'Syrma', 'Iota Virginis', 'Vir', 214004, -6001, 7253),
    (69732, 'Xuange', 'Lambda Bootis', 'Boo', 214096, 46088, 9902),
    (69996, 'Iota Lupi', 'Iota Lupi', 'Lup', 214851, -46058, 33799),
    (70090, 'Psi Centauri', 'Psi Centauri', 'Cen', 215139, -37885, 25885),
    (70497, 'Theta Bootis', 'Theta Bootis', 'Boo', 216299, 51851, 4739),
    (70576, 'Tau2 Lupi', 'Tau2 Lupi', 'Lup', 216545, -45379, 31914),
    (70638, 'Delta Octantis', 'Delta Octantis', 'Oct', 216730, -83668, 29895),
    (70890, 'Proxima Centauri', 'GJ 551', 'Cen', 217429, -62679, 423),
    (71053, 'Rho Bootis', 'Rho Bootis', 'Boo', 217957, 30371, 16012),
    (71075, 'Seginus', 'Gamma Bootis', 'Boo', 218019, 38308, 8679),
    (71121, 'Sigma Lupi', 'Sigma Lupi', 'Lup', 218154, -50457, 57523),
    (71284, 'Sigma Bootis', 'Sigma Bootis', 'Boo', 218670, 29745, 5164),
    (71352, 'Eta Centauri', 'Eta Centauri', 'Cen', 218877, -42158, 30568),
    (71536, 'Rho Lupi', 'Rho Lupi', 'Lup', 219472, -49426, 31604),
    (71681, 'Alpha2 Centauri', 'Alpha2 Centauri', 'Cen', 219896, -60838, 409),
    (71683, 'Alpha Centauri', 'Alpha1 Centauri', 'Cen', 219902, -60834, 432),
    (71795, 'Zeta Bootis', 'Zeta Bootis', 'Boo', 220287, 13728, 17573),
    (71860, 'Alpha Lupi', 'Alpha Lupi', 'Lup', 220482, -47388, 46461),
    (71908, 'Alpha Circini', 'Alpha Circini', 'Cir', 220627, -64975, 5404),
    (71957, 'Mu Virginis', 'Mu Virginis', 'Vir', 220765, -5658, 5959),
    (72105, 'Izar', 'Epsilon Bootis', 'Boo', 221247, 27074, 20258),
    (72370, 'Alpha Apodis', 'Alpha Apodis', 'Aps', 221965, -79045, 44679),
    (72607, 'Kochab', 'Beta Ursae Minoris', 'UMi', 222676, 74156, 13093),
    (72622, 'Zubenelgenubi', 'Alpha2 Librae', 'Lib', 222720, -16042, 7580),
    (72683, 'Omicron Lupi', 'Omicron Lupi', 'Lup', 222910, -43575, 40416),
    (73273, 'Beta Lupi', 'Beta Lupi', 'Lup', 224633, -43134, 38281),
    (73334, 'Kappa Centauri', 'Kappa Centauri', 'Cen', 224790, -42104, 38326),
    (73555, 'Nekkar', 'Beta Bootis', 'Boo', 225487, 40391, 22525),
    (73714, 'Brachium', 'Sigma Librae', 'Lib', 226018, -25282, 28838),
    (74117, 'Lambda Lupi', 'Lambda Lupi', 'Lup', 227211, -45280, 77656),
    (74376, 'Kappa1 Lupi', 'Kappa1 Lupi', 'Lup', 227984, -48738, 18000),
    (74395, 'Zeta Lupi', 'Zeta Lupi', 'Lup', 228071, -52099, 11732),
    (74666, 'Delta Bootis', 'Delta Bootis', 'Boo', 228876, 33315, 12179),
    (74785, 'Zubeneschamali', 'Beta Librae', 'Lib', 229252, -9383, 18511),
    (74824, 'Beta Circini', 'Beta Circini', 'Cir', 229379, -58801, 9965),
    (74911, 'Mu Lupi', 'Mu Lupi', 'Lup', 229633, -47875, 33555),
    (74946, 'Gamma Trianguli Australis', 'Gamma Trianguli Australis', 'TrA', 229727, -68680, 18385),
    (75097, 'Pherkad', 'Gamma Ursae Minoris', 'UMi', 230182, 71834, 48680),
    (75141, 'Delta Lupi', 'Delta Lupi', 'Lup', 230343, -40648, 88389),
    (75177, 'Phi1 Lupi', 'Phi1 Lupi', 'Lup', 230452, -36261, 27501),
    (75264, 'Epsilon Lupi', 'Epsilon Lupi', 'Lup', 230670, -44690, 51202),
    (75411, 'Alkalurops', 'Mu1 Bootis', 'Boo', 231123, 37377, 11313),
    (75458, 'Edasich', 'Iota Draconis', 'Dra', 231232, 58966, 10120),
    (75695, 'Nusakan', 'Beta Coronae Borealis', 'CrB', 231957, 29106, 11181),
    (76127, 'Theta Coronae Borealis', 'Theta Coronae Borealis', 'CrB', 233232, 31359, 37532),
    (76267, 'Alphecca', 'Alpha Coronae Borealis', 'CrB', 233672, 26715, 7505),
    (76276, 'Delta Serpentis', 'Delta Serpentis', 'Ser', 233701, 10539, 22808),
    (76297, 'Gamma Lupi', 'Gamma Lupi', 'Lup', 233785, -41167, 42085),
    (76333, 'Zubenelhakrabi', 'Gamma Librae', 'Lib', 233882, -14790, 16316),
    (76440, 'Epsilon Trianguli Australis', 'Epsilon Trianguli Australis', 'TrA', 234180, -66317, 20170),
    (76470, 'Upsilon Librae', 'Upsilon Librae', 'Lib', 234256, -28135, 22370),
    (76552, 'Omega Lupi', 'Omega Lupi', 'Lup', 234513, -42567, 21860),
    (76600, 'Tau Librae', 'Tau Librae', 'Lib', 234664, -29778, 36688),
    (76952, 'Gamma Coronae Borealis', 'Gamma Coronae Borealis', 'CrB', 235686, 26296, 14606),
    (77055, 'Zeta Ursae Minoris', 'Zeta Ursae Minoris', 'UMi', 236015, 77794, 36896),
    (77070, 'Unukalhai', 'Alpha Serpentis', 'Ser', 236067, 6426, 7396),
    (77233, 'Beta Serpentis', 'Beta Serpentis', 'Ser', 236547, 15422, 15509),
    (77257, 'Lambda Serpentis', 'Lambda Serpentis', 'Ser', 236611, 7353, 3954),
    (77450, 'Gudja', 'Kappa Serpentis', 'Ser', 237185, 18142, 38192),
    (77516, 'Mu Serpentis', 'Mu Serpentis', 'Ser', 237405, -3430, 16961),
    (77622, 'Epsilon Serpentis', 'Epsilon Serpentis', 'Ser', 237704, 4478, 7044),
    (77634, 'Chi Lupi', 'Chi Lupi', 'Lup', 237740, -33627, 19519),
    (77853, 'Theta Librae', 'Theta Librae', 'Lib', 238456, -16729, 16847),
    (77952, 'Beta Trianguli Australis', 'Beta Trianguli Australis', 'TrA', 238786, -63431, 4037),
    (78072, 'Gamma Serpentis', 'Gamma Serpentis', 'Ser', 239113, 15662, 3670),
    (78104, 'Iklil', 'Rho Scorpii', 'Sco', 239221, -29214, 47201),
    (78159, 'Epsilon Coronae Borealis', 'Epsilon Coronae Borealis', 'CrB', 239397, 26878, 22142),
    (78265, 'Fang', 'Pi Scorpii', 'Sco', 239713, -26114, 58556),
    (78384, 'Eta Lupi', 'Eta Lupi', 'Lup', 240031, -38397, 44195),
    (78401, 'Dschubba', 'Delta Scorpii', 'Sco', 240083, -22622, 49120),
    (78527, 'Theta Draconis', 'Theta Draconis', 'Dra', 240472, 58565, 6861),
    (78820, 'Acrab', 'Beta1 Scorpii', 'Sco', 241359, -19805, 40416),
    (78918, 'Theta Lupi', 'Theta Lupi', 'Lup', 241648, -36802, 41443),
    (78933, 'Omega1 Scorpii', 'Omega1 Scorpii', 'Sco', 241702, -20669, 47132),
    (78990, 'Omega2 Scorpii', 'Omega2 Scorpii', 'Sco', 241851, -20869, 29069),
    (79101, 'Phi Herculis', 'Phi Herculis', 'Her', 242192, 44935, 20398),
    (79374, 'Jabbah', 'Nu Scorpii', 'Sco', 242999, -19461, 47406),
    (79593, 'Yed Prior', 'Delta Ophiuchi', 'Oph', 243586, -3694, 17112),
    (79664, 'Delta Trianguli Australis', 'Delta Trianguli Australis', 'TrA', 243859, -63686, 60737),
    (79882, 'Yed Posterior', 'Epsilon Ophiuchi', 'Oph', 244580, -4693, 10645),
    (79992, 'Tau Herculis', 'Tau Herculis', 'Her', 244935, 46313, 30740),
    (80000, 'Gamma2 Normae', 'Gamma2 Normae', 'Nor', 244960, -50156, 12876),
    (80112, 'Alniyat', 'Sigma Scorpii', 'Sco', 245297, -25593, 69692),
    (80170, 'Gamma Herculis', 'Gamma Herculis', 'Her', 245480, 19153, 19265),
    (80331, 'Athebyne', 'Eta Draconis', 'Dra', 245998, 61514, 9208),
    (80343, 'Psi Ophiuchi', 'Psi Ophiuchi', 'Oph', 246026, -20037, 19948),
    (80569, 'Chi Ophiuchi', 'Chi Ophiuchi', 'Oph', 246756, -18456, 52521),
    (80582, 'Epsilon Normae', 'Epsilon Normae', 'Nor', 246796, -47555, 53034),
    (80763, 'Antares', 'Alpha Scorpii', 'Sco', 247352, -26432, 55375),
    (80816, 'Kornephoros', 'Beta Herculis', 'Her', 247555, 21490, 13915),
    (80883, 'Marfik', 'Lambda Ophiuchi', 'Oph', 247728, 1984, 17312),
    (80894, 'Phi Ophiuchi', 'Phi Ophiuchi', 'Oph', 247785, -16613, 24358),
    (80975, 'Omega Ophiuchi', 'Omega Ophiuchi', 'Oph', 248034, -21466, 16864),
    (81065, 'Gamma Apodis', 'Gamma Apodis', 'Aps', 248363, -78897, 15628),
    (81126, 'Sigma Herculis', 'Sigma Herculis', 'Her', 248526, 42437, 31482),
    (81266, 'Paikauhale', 'Tau Scorpii', 'Sco', 248971, -28216, 47406),
    (81377, 'Zeta Ophiuchi', 'Zeta Ophiuchi', 'Oph', 249290, -10567, 36606),
    (81693, 'Zeta Herculis', 'Zeta Herculis', 'Her', 250322, 31603, 3495),
    (81833, 'Eta Herculis', 'Eta Herculis', 'Her', 250724, 38922, 10865),
    (81852, 'Beta Apodis', 'Beta Apodis', 'Aps', 250769, -77517, 15696),
    (82080, 'Epsilon Ursae Minoris', 'Epsilon Ursae Minoris', 'UMi', 251493, 82037, 30397),
    (82273, 'Atria', 'Alpha Trianguli Australis', 'TrA', 252166, -69028, 39061),
    (82363, 'Eta Arae', 'Eta Arae', 'Ara', 252446, -59041, 29923),
    (82396, 'Larawag', 'Epsilon Scorpii', 'Sco', 252541, -34293, 6371),
    (82514, 'Xamidimura', 'Mu1 Scorpii', 'Sco', 252968, -38047, 50101),
    (82545, 'Pipirima', 'Mu2 Scorpii', 'Sco', 253084, -38018, 47406),
    (82673, 'Iota Ophiuchi', 'Iota Ophiuchi', 'Oph', 253502, 10165, 24523),
    (82729, 'Zeta2 Scorpii', 'Zeta2 Scorpii', 'Sco', 253646, -42361, 13231),
    (83000, 'Kappa Ophiuchi', 'Kappa Ophiuchi', 'Oph', 254417, 9375, 9146),
    (83081, 'Zeta Arae', 'Zeta Arae', 'Ara', 254655, -55990, 48608),
    (83153, 'Epsilon1 Arae', 'Epsilon1 Arae', 'Ara', 254896, -53160, 36079),
    (83207, 'Epsilon Herculis', 'Epsilon Herculis', 'Her', 255072, 30926, 15502),
    (83895, 'Aldhibah', 'Zeta Draconis', 'Dra', 257197, 65715, 32846),
    (84012, 'Sabik', 'Eta Ophiuchi', 'Oph', 257595, -15725, 8837),
    (84143, 'Eta Scorpii', 'Eta Scorpii', 'Sco', 258038, -43239, 7348),
    (84345, 'Rasalgethi', 'Alpha1 Herculis', 'Her', 258662, 14390, 35960),
    (84379, 'Sarin', 'Delta Herculis', 'Her', 258758, 24839, 7513),
    (84380, 'Pi Herculis', 'Pi Herculis', 'Her', 258762, 36809, 37662),
    (84405, 'Guniibuu', 'HR 6402', 'Oph', 258837, -26603, 1935),
    (84880, 'Nu Serpentis', 'Nu Serpentis', 'Ser', 260207, -12847, 20321),
    (84893, 'Xi Ophiuchi', 'Xi Ophiuchi', 'Oph', 260252, -21113, 5660),
    (84970, 'Theta Ophiuchi', 'Theta Ophiuchi', 'Oph', 260502, -25000, 43604),
    (85258, 'Beta Arae', 'Beta Arae', 'Ara', 261325, -55530, 64585),
    (85267, 'Gamma Arae', 'Gamma Arae', 'Ara', 261349, -56378, 111316),
    (85355, 'Sigma Ophiuchi', 'Sigma Ophiuchi', 'Oph', 261629, 4140, 90098),
    (85670, 'Rastaban', 'Beta Draconis', 'Dra', 262608, 52301, 38014),
    (85693, 'Maasym', 'Lambda Herculis', 'Her', 262685, 26111, 36937),
    (85696, 'Lesath', 'Upsilon Scorpii', 'Sco', 262691, -37296, 57625),
    (85727, 'Delta Arae', 'Delta Arae', 'Ara', 262775, -60684, 19791),
    (85792, 'Alpha Arae', 'Alpha Arae', 'Ara', 262960, -49876, 26734),
    (85822, 'Yildun', 'Delta Ursae Minoris', 'UMi', 263054, 86586, 17211),
    (85927, 'Shaula', 'Lambda Scorpii', 'Sco', 263402, -37104, 57120),
    (86032, 'Rasalhague', 'Alpha Ophiuchi', 'Oph', 263734, 12560, 4859),
    (86228, 'Sargas', 'Theta Scorpii', 'Sco', 264330, -42998, 30033),
    (86263, 'Xi Serpentis', 'Xi Serpentis', 'Ser', 264397, -15399, 10528),
    (86414, 'Iota Herculis', 'Iota Herculis', 'Her', 264866, 46006, 45489),
    (86565, 'Omicron Serpentis', 'Omicron Serpentis', 'Ser', 265354, -12875, 17321),
    (86670, 'Kappa Scorpii', 'Kappa Scorpii', 'Sco', 265622, -39030, 48319),
    (86742, 'Cebalrai', 'Beta Ophiuchi', 'Oph', 265868, 4567, 8185),
    (86929, 'Eta Pavonis', 'Eta Pavonis', 'Pav', 266433, -64724, 35222),
    (86974, 'Mu Herculis', 'Mu Herculis', 'Her', 266615, 27721, 2711),
    (87073, 'Iota1 Scorpii', 'Iota1 Scorpii', 'Sco', 266896, -40127, 192992),
    (87108, 'Gamma Ophiuchi', 'Gamma Ophiuchi', 'Oph', 266973, 2707, 10279),
    (87261, 'Fuyue', 'HR 6630', 'Sco', 267465, -37043, 12583),
    (87585, 'Grumium', 'Xi Draconis', 'Dra', 268382, 56873, 11255),
    (87808, 'Theta Herculis', 'Theta Herculis', 'Her', 269063, 37251, 75325),
    (87833, 'Eltanin', 'Gamma Draconis', 'Dra', 269152, 51489, 15428),
    (87933, 'Xi Herculis', 'Xi Herculis', 'Her', 269441, 29248, 13681),
    (87937, "Barnard's Star", 'GJ 699', 'Oph', 269452, 4693, 595),
    (87998, 'Nu Herculis', 'Nu Herculis', 'Her', 269626, 30189, 86057),
    (88048, 'Nu Ophiuchi', 'Nu Ophiuchi', 'Oph', 269757, -9774, 15072),
    (88635, 'Alnasl', 'Gamma2 Sagittarii', 'Sgr', 271452, -30424, 9687),
    (88714, 'Theta Arae', 'Theta Arae', 'Ara', 271658, -50091, 81336),
    (88794, 'Omicron Herculis', 'Omicron Herculis', 'Her', 271886, 28762, 33799),
    (88866, 'Pi Pavonis', 'Pi Pavonis', 'Pav', 272145, -63669, 12999),
    (89642, 'Eta Sagittarii', 'Eta Sagittarii', 'Sgr', 274407, -36762, 14593),
    (89826, 'Kappa Lyrae', 'Kappa Lyrae', 'Lyr', 274965, 36065, 25166),
    (89908, 'Phi Draconis', 'Phi Draconis', 'Dra', 275189, 71338, 30284),
    (89931, 'Kaus Media', 'Delta Sagittarii', 'Sgr', 275249, -29828, 34771),
    (89937, 'Chi Draconis', 'Chi Draconis', 'Dra', 275264, 72733, 2628),
    (89962, 'Eta Serpentis', 'Eta Serpentis', 'Ser', 275328, -2899, 6048),
    (90098, 'Xi Pavonis', 'Xi Pavonis', 'Pav', 275807, -61494, 46862),
    (90185, 'Kaus Australis', 'Epsilon Sagittarii', 'Sgr', 276043, -34385, 14330),
    (90422, 'Alpha Telescopii', 'Alpha Telescopii', 'Tel', 276743, -45968, 27782),
    (90496, 'Kaus Borealis', 'Lambda Sagittarii', 'Sgr', 276993, -25422, 7818),
    (90568, 'Zeta Telescopii', 'Zeta Telescopii', 'Tel', 277208, -49071, 12622),
    (91117, 'Alpha Scuti', 'Alpha Scuti', 'Sct', 278802, -8244, 19912),
    (91262, 'Vega', 'Alpha Lyrae', 'Lyr', 279235, 38784, 2504),
    (91792, 'Zeta Pavonis', 'Zeta Pavonis', 'Pav', 280759, -71428, 21846),
    (91971, 'Zeta1 Lyrae', 'Zeta1 Lyrae', 'Lyr', 281193, 37605, 15613),
    (92041, 'Phi Sagittarii', 'Phi Sagittarii', 'Sgr', 281414, -26991, 23929),
    (92175, 'Beta Scuti', 'Beta Scuti', 'Sct', 281794, -4748, 91617),
    (92420, 'Sheliak', 'Beta Lyrae', 'Lyr', 282520, 33363, 96211),
    (92609, 'Lambda Pavonis', 'Lambda Pavonis', 'Pav', 283054, -62188, 143051),
    (92791, 'Delta2 Lyrae', 'Delta2 Lyrae', 'Lyr', 283626, 36899, 73624),
    (92855, 'Nunki', 'Sigma Sagittarii', 'Sgr', 283816, -26297, 22776),
    (93015, 'Kappa Pavonis', 'Kappa Pavonis', 'Pav', 284238, -67233, 50024),
    (93085, 'Xi2 Sagittarii', 'Xi2 Sagittarii', 'Sgr', 284432, -21107, 36524),
    (93194, 'Sulafat', 'Gamma Lyrae', 'Lyr', 284736, 32690, 62007),
    (93244, 'Epsilon Aquilae', 'Epsilon Aquilae', 'Aql', 284906, 15068, 15494),
    (93506, 'Ascella', 'Zeta Sagittarii', 'Sgr', 285653, -29880, 8820),
    (93683, 'Omicron Sagittarii', 'Omicron Sagittarii', 'Sgr', 286171, -21741, 14205),
    (93747, 'Okab', 'Zeta Aquilae', 'Aql', 286353, 13863, 8303),
    (93805, 'Lambda Aquilae', 'Lambda Aquilae', 'Aql', 286562, -4883, 12368),
    (93864, 'Tau Sagittarii', 'Tau Sagittarii', 'Sgr', 286735, -27670, 12161),
    (94114, 'Meridiana', 'Alpha Coronae Australis', 'CrA', 287368, -37904, 12535),
    (94141, 'Albaldah', 'Pi Sagittarii', 'Sgr', 287441, -21024, 50962),
    (94160, 'Beta Coronae Australis', 'Beta Coronae Australis', 'CrA', 287507, -39341, 47406),
    (94376, 'Altais', 'Delta Draconis', 'Dra', 288139, 67662, 9742),
    (94481, 'Aladfar', 'Eta Lyrae', 'Lyr', 288440, 39146, 138790),
    (94648, 'Tau Draconis', 'Tau Draconis', 'Dra', 288888, 73355, 14639),
    (94713, 'Theta Lyrae', 'Theta Lyrae', 'Lyr', 289092, 38134, 83203),
    (94779, 'Kappa Cygni', 'Kappa Cygni', 'Cyg', 289276, 53368, 12416),
    (95168, 'Rho1 Sagittarii', 'Rho1 Sagittarii', 'Sgr', 290418, -17847, 12696),
    (95241, 'Arkab Prior', 'Beta1 Sagittarii', 'Sgr', 290660, -44459, 31361),
    (95294, 'Arkab Posterior', 'Beta2 Sagittarii', 'Sgr', 290805, -44800, 13417),
    (95347, 'Rukbat', 'Alpha Sagittarii', 'Sgr', 290972, -40616, 18180),
    (95501, 'Delta Aquilae', 'Delta Aquilae', 'Aql', 291375, 3115, 5064),
    (95771, 'Anser', 'Alpha Vulpeculae', 'Vul', 292176, 24665, 29732),
    (95853, 'Iota2 Cygni', 'Iota2 Cygni', 'Cyg', 292426, 51730, 12134),
    (95947, 'Albireo', 'Beta1 Cygni', 'Cyg', 292680, 27960, 43430),
    (96100, 'Alsafi', 'HR 7462', 'Dra', 293090, 69661, 1877),
    (96229, 'Mu Aquilae', 'Mu Aquilae', 'Aql', 293522, 7379, 10761),
    (96441, 'Theta Cygni', 'Theta Cygni', 'Cyg', 294111, 50221, 5980),
    (96468, 'Iota Aquilae', 'Iota Aquilae', 'Aql', 294180, -1287, 39107),
    (96757, 'Sham', 'Alpha Sagittae', 'Sge', 295024, 18014, 42524),
    (96837, 'Beta Sagittae', 'Beta Sagittae', 'Sge', 295262, 17476, 43956),
    (97165, 'Fawaris', 'Delta Cygni', 'Cyg', 296244, 45131, 16498),
    (97278, 'Tarazed', 'Gamma Aquilae', 'Aql', 296565, 10613, 39486),
    (97365, 'Delta Sagittae', 'Delta Sagittae', 'Sge', 296847, 18534, 59409),
    (97433, 'Epsilon Draconis', 'Epsilon Draconis', 'Dra', 297043, 70268, 14798),
    (97649, 'Altair', 'Alpha Aquilae', 'Aql', 297696, 8868, 1673),
    (98032, 'Iota Sagittarii', 'Iota Sagittarii', 'Sgr', 298815, -41868, 18180),
    (98036, 'Alshain', 'Beta Aquilae', 'Aql', 298828, 6407, 4468),
    (98110, 'Eta Cygni', 'Eta Cygni', 'Cyg', 299077, 35083, 13494),
    (98337, 'Gamma Sagittae', 'Gamma Sagittae', 'Sge', 299689, 19492, 25844),
    (98412, 'Theta1 Sagittarii', 'Theta1 Sagittarii', 'Sgr', 299934, -35276, 51853),
    (98495, 'Epsilon Pavonis', 'Epsilon Pavonis', 'Pav', 300148, -72911, 10508),
    (99240, 'Delta Pavonis', 'Delta Pavonis', 'Pav', 302182, -66182, 1992),
    (99255, 'Kappa Cephei', 'Kappa Cephei', 'Cep', 302222, 77711, 31482),
    (99473, 'Theta Aquilae', 'Theta Aquilae', 'Aql', 302826, -821, 28635),
    (100027, 'Alpha1 Capricorni', 'Alpha1 Capricorni', 'Cap', 304412, -12508, 56921),
    (100064, 'Algedi', 'Alpha2 Capricorni', 'Cap', 304514, -12545, 10583),
    (100345, 'Dabih', 'Beta Capricorni', 'Cap', 305253, -14781, 32681),
    (100453, 'Sadr', 'Gamma Cygni', 'Cyg', 305557, 40257, 183234),
    (100751, 'Peacock', 'Alpha Pavonis', 'Pav', 306412, -56735, 17881),
    (101093, 'Theta Cephei', 'Theta Cephei', 'Cep', 307395, 62994, 13647),
    (101421, 'Aldulfin', 'Epsilon Delphini', 'Del', 308303, 11303, 33045),
    (101769, 'Rotanev', 'Beta Delphini', 'Del', 309387, 14595, 10088),
    (101772, 'Alpha Indi', 'Alpha Indi', 'Ind', 309392, -47292, 9833),
    (101958, 'Sualocin', 'Alpha Delphini', 'Del', 309910, 15912, 25382),
    (102098, 'Deneb', 'Alpha Cygni', 'Cyg', 310358, 45280, 261577),
    (102281, 'Delta Delphini', 'Delta Delphini', 'Del', 310865, 15075, 22324),
    (102395, 'Beta Pavonis', 'Beta Pavonis', 'Pav', 311240, -66203, 13511),
    (102422, 'Eta Cephei', 'Eta Cephei', 'Cep', 311322, 61839, 4653),
    (102485, 'Psi Capricorni', 'Psi Capricorni', 'Cap', 311524, -25271, 4787),
    (102488, 'Aljanah', 'Epsilon Cygni', 'Cyg', 311553, 33970, 7271),
    (102532, 'Gamma2 Delphini', 'Gamma2 Delphini', 'Del', 311665, 16124, 12632),
    (102618, 'Albali', 'Epsilon Aquarii', 'Aqr', 311919, -9496, 20774),
    (102978, 'Omega Capricorni', 'Omega Capricorni', 'Cap', 312955, -26919, 84278),
    (103227, 'Beta Indi', 'Beta Indi', 'Ind', 313703, -58454, 61193),
    (103413, 'Nu Cygni', 'Nu Cygni', 'Cyg', 314293, 41167, 37446),
    (104060, 'Xi Cygni', 'Xi Cygni', 'Cyg', 316233, 43928, 84278),
    (104139, 'Theta Capricorni', 'Theta Capricorni', 'Cap', 316487, -17233, 16219),
    (104214, '61 Cygni', '61 Cygni', 'Cyg', 316725, 38749, 1137),
    (104732, 'Zeta Cygni', 'Zeta Cygni', 'Cyg', 318234, 30227, 14311),
    (104858, 'Delta Equulei', 'Delta Equulei', 'Equ', 318620, 10007, 6030),
    (104887, 'Tau Cygni', 'Tau Cygni', 'Cyg', 318698, 38045, 6635),
    (104987, 'Kitalpha', 'Alpha Equulei', 'Equ', 318956, 5248, 19029),
    (105102, 'Sigma Cygni', 'Sigma Cygni', 'Cyg', 319354, 39395, 288634),
    (105138, 'Upsilon Cygni', 'Upsilon Cygni', 'Cyg', 319479, 34897, 64204),
    (105199, 'Alderamin', 'Alpha Cephei', 'Cep', 319645, 62586, 4905),
    (105319, 'Theta Indi', 'Theta Indi', 'Ind', 319967, -53449, 9878),
    (105515, 'Iota Capricorni', 'Iota Capricorni', 'Cap', 320562, -16835, 19672),
    (105858, 'Gamma Pavonis', 'Gamma Pavonis', 'Pav', 321611, -65366, 3021),
    (105881, 'Zeta Capricorni', 'Zeta Capricorni', 'Cap', 321667, -22411, 38553),
    (106032, 'Alfirk', 'Beta Cephei', 'Cep', 322165, 70561, 68520),
    (106278, 'Sadalsuud', 'Beta Aquarii', 'Aqr', 322890, -5571, 53733),
    (106481, 'Rho Cygni', 'Rho Cygni', 'Cyg', 323495, 45592, 12359),
    (106985, 'Nashira', 'Gamma Capricorni', 'Cap', 325023, -16662, 15703),
    (107089, 'Nu Octantis', 'Nu Octantis', 'Oct', 325369, -77390, 6914),
    (107315, 'Enif', 'Epsilon Pegasi', 'Peg', 326046, 9875, 68955),
    (107354, 'Kappa Pegasi', 'Kappa Pegasi', 'Peg', 326161, 25645, 11162),
    (107380, 'Iota Piscis Austrini', 'Iota Piscis Austrini', 'PsA', 326237, -33026, 20423),
    (107418, 'Nu Cephei', 'Nu Cephei', 'Cep', 326362, 61121, 679492),
    (107533, 'Pi2 Cygni', 'Pi2 Cygni', 'Cyg', 326698, 49310, 110561),
    (107556, 'Deneb Algedi', 'Delta Capricorni', 'Cap', 326760, -16127, 3870),
    (108085, 'Aldhanab', 'Gamma Gruis', 'Gru', 328482, -37365, 21110),
    (108431, 'Delta Indi', 'Delta Indi', 'Ind', 329479, -54993, 18809),
    (108870, 'Epsilon Indi', 'Epsilon Indi', 'Ind', 330840, -56786, 1181),
    (108917, 'Kurhah', 'Xi Cephei', 'Cep', 330948, 64628, 9652),
    (109074, 'Sadalmelik', 'Alpha Aquarii', 'Aqr', 331446, -320, 52353),
    (109111, 'Lambda Gruis', 'Lambda Gruis', 'Gru', 331529, -39543, 24214),
    (109139, 'Iota Aquarii', 'Iota Aquarii', 'Aqr', 331609, -13870, 17516),
    (109176, 'Iota Pegasi', 'Iota Pegasi', 'Peg', 331753, 25345, 3825),
    (109268, 'Alnair', 'Alpha Gruis', 'Gru', 332058, -46961, 10101),
    (109285, 'Mu Piscis Austrini', 'Mu Piscis Austrini', 'PsA', 332096, -32988, 13584),
    (109410, 'Pi2 Pegasi', 'Pi2 Pegasi', 'Peg', 332497, 33178, 26303),
    (109427, 'Biham', 'Theta Pegasi', 'Peg', 332550, 6198, 9229),
    (109492, 'Zeta Cephei', 'Zeta Cephei', 'Cep', 332714, 58201, 83630),
    (109857, 'Epsilon Cephei', 'Epsilon Cephei', 'Cep', 333759, 57044, 8545),
    (110003, 'Ancha', 'Theta Aquarii', 'Aqr', 334208, -7783, 18745),
    (110130, 'Alpha Tucanae', 'Alpha Tucanae', 'Tuc', 334625, -60260, 19973),
    (110395, 'Sadachbia', 'Gamma Aquarii', 'Aqr', 335414, -1387, 16373),
    (110538, 'Beta Lacertae', 'Beta Lacertae', 'Lac', 335890, 52229, 16996),
    (110838, 'Delta Tucanae', 'Delta Tucanae', 'Tuc', 336833, -64966, 25089),
    (110960, 'Zeta2 Aquarii', 'Zeta2 Aquarii', 'Aqr', 337208, -20, 9188),
    (110991, 'Delta Cephei', 'Delta Cephei', 'Cep', 337293, 58415, 86514),
    (110997, 'Delta1 Gruis', 'Delta1 Gruis', 'Gru', 337317, -43496, 30945),
    (111043, 'Delta2 Gruis', 'Delta2 Gruis', 'Gru', 337439, -43749, 33012),
    (111169, 'Alpha Lacertae', 'Alpha Lacertae', 'Lac', 337823, 50282, 10260),
    (111188, 'Beta Piscis Austrini', 'Beta Piscis Austrini', 'PsA', 337876, -32346, 14280),
    (111497, 'Eta Aquarii', 'Eta Aquarii', 'Aqr', 338839, -117, 16786),
    (111954, 'Epsilon Piscis Austrini', 'Epsilon Piscis Austrini', 'PsA', 340164, -27044, 48680),
    (112029, 'Homam', 'Zeta Pegasi', 'Peg', 340366, 10831, 20436),
    (112122, 'Tiaki', 'Beta Gruis', 'Gru', 340667, -46885, 17697),
    (112158, 'Matar', 'Eta Pegasi', 'Peg', 340751, 30221, 21429),
    (112405, 'Beta Octantis', 'Beta Octantis', 'Oct', 341515, -81382, 14927),
    (112440, 'Lambda Pegasi', 'Lambda Pegasi', 'Peg', 341633, 23566, 36524),
    (112447, 'Xi Pegasi', 'Xi Pegasi', 'Peg', 341673, 12173, 5315),
    (112623, 'Epsilon Gruis', 'Epsilon Gruis', 'Gru', 342139, -51317, 12892),
    (112716, 'Tau2 Aquarii', 'Tau2 Aquarii', 'Aqr', 342398, -13593, 31758),
    (112724, 'Iota Cephei', 'Iota Cephei', 'Cep', 342420, 66200, 11529),
    (112748, 'Sadalbari', 'Mu Pegasi', 'Peg', 342501, 24602, 10610),
    (112948, 'Gamma Piscis Austrini', 'Gamma Piscis Austrini', 'PsA', 343131, -32876, 21543),
    (112961, 'Lambda Aquarii', 'Lambda Aquarii', 'Aqr', 343154, -7580, 38507),
    (113136, 'Skat', 'Delta Aquarii', 'Aqr', 343663, -15821, 16059),
    (113246, 'Delta Piscis Austrini', 'Delta Piscis Austrini', 'PsA', 343987, -32540, 15414),
    (113368, 'Fomalhaut', 'Alpha Piscis Austrini', 'PsA', 344413, -29622, 2513),
    (113638, 'Zeta Gruis', 'Zeta Gruis', 'Gru', 345220, -52754, 10886),
    (113726, 'Omicron Andromedae', 'Omicron Andromedae', 'And', 345480, 42326, 68665),
    (113881, 'Scheat', 'Beta Pegasi', 'Peg', 345944, 28083, 19601),
    (113889, 'Fumalsamakah', 'HR 8773', 'Psc', 345969, 3820, 40821),
    (113963, 'Markab', 'Alpha Pegasi', 'Peg', 346190, 15205, 13334),
    (114131, 'Theta Gruis', 'Theta Gruis', 'Gru', 346720, -43520, 13189),
    (114222, 'Pi Cephei', 'Pi Cephei', 'Cep', 346974, 75387, 24671),
    (114421, 'Iota Gruis', 'Iota Gruis', 'Gru', 347590, -45247, 18323),
    (114724, 'Phi Aquarii', 'Phi Aquarii', 'Aqr', 348581, -6049, 20208),
    (114855, 'Psi1 Aquarii', 'Psi1 Aquarii', 'Aqr', 348973, -9088, 14982),
    (114971, 'Gamma Piscium', 'Gamma Piscium', 'Psc', 349291, 3282, 13797),
    (114996, 'Gamma Tucanae', 'Gamma Tucanae', 'Tuc', 349357, -58236, 7520),
    (115033, 'Psi2 Aquarii', 'Psi2 Aquarii', 'Aqr', 349476, -9183, 40167),
    (115102, 'Gamma Sculptoris', 'Gamma Sculptoris', 'Scl', 349706, -32532, 18221),
    (115623, 'Alkarab', 'Upsilon Pegasi', 'Peg', 351345, 23404, 17041),
    (115830, 'Theta Piscium', 'Theta Piscium', 'Psc', 351992, 6379, 14852),
    (116231, 'Beta Sculptoris', 'Beta Sculptoris', 'Scl', 353243, -37818, 17404),
    (116584, 'Lambda Andromedae', 'Lambda Andromedae', 'And', 354391, 46458, 8613),
    (116631, 'Iota Andromedae', 'Iota Andromedae', 'And', 354534, 43268, 49947),
    (116727, 'Errai', 'Gamma Cephei', 'Cep', 354837, 77632, 4600),
    (116771, 'Iota Piscium', 'Iota Piscium', 'Psc', 354988, 5626, 4473),
    (116805, 'Kappa Andromedae', 'Kappa Andromedae', 'And', 355102, 44334, 16838),
    (116928, 'Lambda Piscium', 'Lambda Piscium', 'Psc', 355512, 1780, 10662),
    (116971, 'Omega2 Aquarii', 'Omega2 Aquarii', 'Aqr', 355681, -14545, 14852),
    (118268, 'Omega Piscium', 'Omega Piscium', 'Psc', 359828, 6863, 10434),
    (118322, 'Epsilon Tucanae', 'Epsilon Tucanae', 'Tuc', 359979, -65577, 37318),
]

# Stick figures: constellation -> pairs of Hipparcos numbers
FIGURES = {
    'Ori': [(24436, 25930), (24436, 27366), (25336, 25930), (25336, 26207), (25930, 26311), (26207, 27989), (26311, 26727), (26727, 27366), (26727, 27989)],
    'UMa': [(53910, 54061), (53910, 58001), (54061, 59774), (58001, 59774), (59774, 62956), (62956, 65378), (65378, 67301)],
    'UMi': [(11767, 85822), (72607, 75097), (72607, 77055), (77055, 82080), (82080, 85822)],
    'Cas': [(746, 3179), (3179, 4427), (4427, 6686), (6686, 8886)],
    'Cyg': [(95947, 100453), (97165, 100453), (100453, 102098), (100453, 102488)],
    'Lyr': [(91262, 91971), (91971, 92420), (91971, 92791), (92420, 93194), (92791, 93194)],
    'Aql': [(97278, 97649), (97649, 98036)],
    'Sco': [(78265, 78401), (78401, 78820), (78401, 80112), (80112, 80763), (80763, 81266), (81266, 82396), (82396, 82514), (82514, 82729), (82729, 84143), (84143, 86228), (85696, 85927), (85927, 86670), (86228, 87073), (86670, 87073)],
    'Leo': [(47908, 48455), (48455, 50335), (49583, 49669), (49583, 50583), (50335, 50583), (50583, 54872), (54872, 54879), (54872, 57632), (54879, 57632)],
    'Cru': [(59747, 62434), (60718, 61084)],
    'Boo': [(69673, 71053), (69673, 72105), (71053, 71075), (71075, 73555), (72105, 74666), (73555, 74666)],
    'CMa': [(30324, 32349), (32349, 34045), (32349, 34444), (33579, 34444), (34444, 35904)],
    'Peg': [(1067, 113881), (1067, 113963), (113881, 113963)],
    'And': [(677, 5447), (5447, 9640)],
    'Aur': [(23015, 24608), (24608, 28360), (28360, 28380)],
    'Tau': [(20205, 20455), (20205, 21421), (20455, 20889), (21421, 26451)],
    'Sgr': [(88635, 90185), (89931, 90185), (89931, 90496), (90185, 93506), (90496, 92041), (92041, 93506)],
    'CMi': [(36188, 37279)],
    'Tri': [(8796, 10064), (8796, 10670), (10064, 10670)],
    'Cep': [(105199, 106032), (105199, 109492), (106032, 116727), (109492, 112724), (112724, 116727)],
    'Her': [(81693, 81833), (81693, 83207), (81833, 84380), (83207, 84380)],
    'CrB': [(75695, 76127), (75695, 76267), (76267, 76952)],
}


# Least distance between two stars on the map, in map units (task 161, 2026-11-26: 60 for now;
# the star-map and system-view scales follow in a later card). In the widest map (every star,
# 4.8 frame units per map unit) two stars 60 apart are 2 765 system units apart at the
# hand-over to the star map: their discs (1 344 wide at whole-system zoom) stand apart, their
# glows (2 600 wide) almost touch.
MIN_SPACING = 60
# A star that cannot be cleared by changing its map radius within this share (percent) of it
# moves to the nearest free point instead.
RADIAL_SLACK_PCT = 25

# What curate() does to the catalogue at MIN_SPACING 60 (`curation_summary()` prints it when the
# seed is generated; docs/database.md "Star catalogue" lists the same):
# - 4 rows merged, 745 -> 741 systems: Alpha2 Centauri into Alpha Centauri, Delta3 Tauri into
#   Secunda Hyadum, Theta1 Tauri into Chamukuy, Delta2 Gruis into Delta Gruis (the Delta1 row);
# - 26 stars moved, in map units, along their line of sight unless marked: Kappa Phoenicis 17,
#   Gamma Persei 2, Iota Persei 29, Taygeta 48, Maia 6, Merope 56, Alcyone 119, Atlas 119,
#   Secunda Hyadum 16, Upsilon Tauri 59, Ain 21, Chamukuy 36, Phi1 Orionis 39, Rho Geminorum 5,
#   Gamma Centauri 53, Alcor 38, Arcturus 27, Alpha Centauri 51 (to the nearest free point,
#   14 degrees off its direction: Proxima Centauri, named and earlier in Hipparcos order, keeps
#   its place, and no radius within 25 % clears it), Kappa Centauri 47, Epsilon Serpentis 26,
#   Beta Apodis 49, Shaula 44, Eta Sagittarii 8, Iota2 Cygni 23, Beta Sagittae 22, Eta Cephei 20.


def component_base(designation):
    """The designation without its component number ("Alpha2 Centauri" -> "Alpha Centauri"),
    or None when it is no Bayer designation (Flamsteed numbers, HR and GJ numbers). A bare
    letter ("Alpha Centauri") is its own base, so it groups with the numbered components."""
    m = re.fullmatch(r'([A-Z][a-z]+)\d* (.+)', designation)
    return f'{m.group(1)} {m.group(2)}' if m else None


def _clear(p, placed, spacing):
    """True when the integer point `p` is at least `spacing` from every point of `placed`."""
    limit = spacing * spacing
    return all((p[0] - q[0]) ** 2 + (p[1] - q[1]) ** 2 + (p[2] - q[2]) ** 2 >= limit for q in placed)


def _shell_directions(count=512):
    """`count` fixed, evenly spread unit directions (a Fibonacci sphere), in a fixed order."""
    golden = math.pi * (3 - math.sqrt(5))
    out = []
    for i in range(count):
        z = 1 - (2 * i + 1) / count
        r = math.sqrt(1 - z * z)
        out.append((r * math.cos(golden * i), r * math.sin(golden * i), z))
    return out


def curate(stars=None, spacing=MIN_SPACING):
    """The curated catalogue (task 161): a list of (hip, name, designation, abbr, ra, dec, dist,
    x, y, z) in Hipparcos order, a map from each dropped Hipparcos number to the one kept in its
    place, and the moves as (hip, name, how, change, distance moved) in placement order: `how` is
    'radius' (`change` the radius step, + farther) or 'point' (`change` the shell's radius)."""
    stars = sorted(STARS if stars is None else stars)
    by_hip = {s[0]: s for s in stars}
    true_pos = {s[0]: position(*s[4:7]) for s in stars}

    def dist2(a, b):
        p, q = true_pos[a], true_pos[b]
        return sum((p[k] - q[k]) ** 2 for k in range(3))

    def proper(hip):
        return by_hip[hip][1] != by_hip[hip][2]

    # 1. Components of one multiple closer than the spacing are one system (union by pairs).
    parent = {h: h for h in by_hip}

    def root(h):
        while parent[h] != h:
            h = parent[h]
        return h
    groups = {}
    for s in stars:
        base = component_base(s[2])
        if base is not None:
            groups.setdefault((base, s[3]), []).append(s[0])
    for members in groups.values():
        for i, a in enumerate(members):
            for b in members[i + 1:]:
                if dist2(a, b) < spacing * spacing:
                    ra, rb = root(a), root(b)
                    if ra != rb:
                        parent[max(ra, rb)] = min(ra, rb)
    merged = {}
    kept_name = {}
    clusters = {}
    for h in by_hip:
        clusters.setdefault(root(h), []).append(h)
    for members in clusters.values():
        if len(members) < 2:
            continue
        keep = min(members, key=lambda h: (not proper(h), h))
        kept_name[keep] = by_hip[keep][1] if proper(keep) else component_base(by_hip[keep][2])
        for h in members:
            if h != keep:
                merged[h] = keep

    # 2. Every other close pair moves apart: named stars first, then by Hipparcos number.
    order = sorted((h for h in by_hip if h not in merged), key=lambda h: (not proper(h), h))
    placed = {}
    moves = []
    shells = None
    for h in order:
        p = true_pos[h]
        others = list(placed.values())
        if not _clear(p, others, spacing):
            ra, dec, dist = by_hip[h][4:7]
            radius = RADIAL_SCALE * math.sqrt(dist / 100)
            ra_r, dec_r = math.radians(ra / 1000), math.radians(dec / 1000)
            u = (math.cos(dec_r) * math.cos(ra_r), math.cos(dec_r) * math.sin(ra_r), math.sin(dec_r))
            found = None
            # Along its line of sight: the smallest change of radius, farther before nearer.
            for step in range(1, int(radius * RADIAL_SLACK_PCT / 100) + 1):
                for r in (radius + step, radius - step):
                    q = tuple(round(r * c) for c in u)
                    if _clear(q, others, spacing):
                        found = (q, 'radius', step if r > radius else -step)
                        break
                if found:
                    break
            if found is None:
                # The nearest free point: growing shells of fixed directions about the true place.
                shells = shells or _shell_directions()
                shell = 1
                while found is None:
                    for d in shells:
                        q = tuple(round(p[k] + shell * d[k]) for k in range(3))
                        if _clear(q, others, spacing):
                            found = (q, 'point', shell)
                            break
                    shell += 1
            p = found[0]
            moves.append((h, kept_name.get(h, by_hip[h][1]), found[1], found[2], math.dist(p, true_pos[h])))
        placed[h] = p
    rows = []
    for h in sorted(placed):
        hip, name, designation, abbr, ra, dec, dist = by_hip[h]
        rows.append((hip, kept_name.get(h, name), designation, abbr, ra, dec, dist, *placed[h]))
    return rows, merged, moves


def curation_summary(spacing=MIN_SPACING):
    """A few lines on what `curate()` merged and moved, printed when the seed is generated."""
    rows, merged, moves = curate(spacing=spacing)
    names = {s[0]: s[1] for s in STARS}
    kept = {r[0]: r[1] for r in rows}
    lines = [f'stars: {len(STARS)} in the catalogue, {len(rows)} systems; minimum spacing {spacing}']
    for dropped, into in sorted(merged.items()):
        lines.append(f'  merged {names[dropped]} into {kept[into]}')
    for hip, name, how, change, moved in moves:
        what = f'radius {change:+d}' if how == 'radius' else f'shell {change}'
        lines.append(f'  moved {name} (HIP {hip}) by {moved:.0f} ({what})')
    return '\n'.join(lines)


def position(ra_mdeg, dec_mdeg, dist_cly):
    """Integer map position (x toward RA 0h, y toward RA 6h, z toward the north pole)."""
    radius = RADIAL_SCALE * math.sqrt(dist_cly / 100)
    ra, dec = math.radians(ra_mdeg / 1000), math.radians(dec_mdeg / 1000)
    return (round(radius * math.cos(dec) * math.cos(ra)),
            round(radius * math.cos(dec) * math.sin(ra)),
            round(radius * math.sin(dec)))


def galactic_longitude(ra_mdeg, dec_mdeg):
    """Galactic longitude seen from the Sun in integer milli-degrees, 0..359999 (task 89): the
    quadrant a star lies in (I 0-90, II 90-180, III 180-270, IV 270-360 degrees)."""
    pole_ra, pole_dec = math.radians(GALACTIC_POLE_RA_DEG), math.radians(GALACTIC_POLE_DEC_DEG)
    ra, dec = math.radians(ra_mdeg / 1000), math.radians(dec_mdeg / 1000)
    across = math.cos(dec) * math.sin(ra - pole_ra)
    up = math.sin(dec) * math.cos(pole_dec) - math.cos(dec) * math.sin(pole_dec) * math.cos(ra - pole_ra)
    lon = (GALACTIC_NCP_LON_DEG - math.degrees(math.atan2(across, up))) % 360
    return round(lon * 1000) % 360000


def star_sql(val):
    """INSERT statements for constellation, star and constellation_line from the curated catalogue
    (`curate()`); `val` quotes a value."""
    rows, merged, _ = curate()
    used = sorted({s[3] for s in rows})
    sql = []
    ids = {abbr: i + 1 for i, abbr in enumerate(used)}
    for abbr in used:
        name, genitive = CONSTELLATIONS[abbr]
        sql.append(f"INSERT INTO constellation(id,abbr,name,genitive) VALUES ({ids[abbr]},{val(abbr)},{val(name)},{val(genitive)});")
    star_id = {}
    for i, (hip, name, designation, abbr, ra, dec, dist, x, y, z) in enumerate(rows):
        star_id[hip] = i + 1
        glon = galactic_longitude(ra, dec)
        sql.append("INSERT INTO star(id,hip,name,designation,constellation_id,ra_mdeg,dec_mdeg,dist_cly,x,y,z,glon_mdeg) VALUES "
                   f"({i + 1},{hip},{val(name)},{val(designation)},{ids[abbr]},{ra},{dec},{dist},{x},{y},{z},{glon});")
    for abbr in sorted(FIGURES):
        # A line to a merged star goes to the one kept in its place; a line from a star to
        # itself is dropped.
        lines = {tuple(sorted(merged.get(h, h) for h in e)) for e in FIGURES[abbr]}
        for a, b in sorted(e for e in lines if e[0] != e[1]):
            sql.append(f"INSERT INTO constellation_line(constellation_id,star_a,star_b) VALUES ({ids[abbr]},{star_id[a]},{star_id[b]});")
    return sql
