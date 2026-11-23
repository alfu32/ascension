"""The instruments the renderer plays, and the sample sets they come from.

Two CC0 libraries by the same studio, pinned to fixed commits (README.md):

- ``vsco``: VS Chamber Orchestra 2, Community Edition: string sections, winds, brass, timpani,
  harp, percussion. Mapped by its own SFZ files.
- ``vcsl``: Versilian Community Sample Library: a folk harp (the lyre), frame drums, finger
  cymbals. Mapped here by file name.

A third source, the FluidR3 General MIDI bank (MIT), gives the colour instruments neither library
has (voices, shawm, a drone pipe, synth, electric guitar). It is read from the system's sound font.
"""

VSCO_REPO = "sgossner/VSCO-2-CE"
VSCO_COMMIT = "28092772094b2d9f1148d84cea97f4545b8c687d"  # tag 1.1.0
VCSL_REPO = "sgossner/VCSL"
VCSL_COMMIT = "c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e"  # master, 2026-01-14
FLUID_SF2 = "/usr/share/sounds/sf2/FluidR3_GM.sf2"
FLUID_SHA256 = "74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0"

# Articulations: "sus" (long, legato), "nv" (long, no vibrato), "short" (spiccato, staccato),
# "pizz", "trem", "mute", "roll", "hit". The renderer picks one from the GM program in force
# (pizzicato 45, tremolo 44, muted trumpet 59, timpani roll on a held note) and the note length.
VSCO_PATCHES = {
    "violins": {"sus": "ViolinEnsSusVib.sfz", "short": "ViolinEnsSpic.sfz",
                "pizz": "ViolinEnsPizz.sfz", "trem": "ViolinEnsTrem.sfz"},
    "violas": {"sus": "ViolaEnsSusVib.sfz", "short": "ViolaEnsSpic.sfz",
               "pizz": "ViolaEnsPizz.sfz", "trem": "ViolaEnsTrem.sfz"},
    "cellos": {"sus": "CelloEnsSusVib.sfz", "short": "CelloEnsSpic.sfz",
               "pizz": "CelloEnsPizz.sfz", "trem": "CelloEnsTrem.sfz"},
    "basses": {"sus": "ContrabassSusVB.sfz", "nv": "ContrabassSusNV.sfz",
               "short": "ContrabassSpic.sfz", "pizz": "ContrabassPizz.sfz",
               "trem": "ContrabassTrem.sfz"},
    "solo_violin": {"sus": "SViolinVib.sfz", "short": "SViolinSpic.sfz",
                    "pizz": "SViolinPizz.sfz"},
    "harp": {"pluck": "Harp.sfz"},
    "flute": {"sus": "FluteSusVib.sfz", "nv": "FluteSusNV.sfz", "short": "FluteStac.sfz"},
    "piccolo": {"sus": "PiccoloSus.sfz", "short": "PiccoloStac.sfz"},
    "oboe": {"sus": "OboeSusVib.sfz", "nv": "OboeSusNV.sfz", "short": "OboeStac.sfz"},
    "clarinet": {"sus": "ClarinetSus.sfz", "short": "ClarinetStac.sfz"},
    "bassoon": {"sus": "BassoonSus.sfz", "short": "BassoonStac.sfz"},
    "horn": {"sus": "FHornSus.sfz", "short": "FHornStac.sfz"},
    "trumpet": {"sus": "TrumpetSus.sfz", "short": "TrumpetStac.sfz",
                "mute": "TrumpetStraightMuteSus.sfz"},
    "trombone": {"sus": "TromboneSus.sfz", "short": "TromboneStac.sfz"},
    "tuba": {"sus": "TubaSus.sfz", "short": "TubaStac.sfz"},
    "timpani": {"hit": "Timpani.sfz", "roll": "TimpaniRolls.sfz"},
    "bells": {"hit": "TubularBells.sfz"},
    "glock": {"hit": "Glockenspiel.sfz"},
    "perc": {"hit": "GM-StylePerc.sfz"},
}

# GM percussion keys used from GM-StylePerc.sfz (bass drum, snare, gong, suspended cymbal,
# triangle). The rest of that kit is not fetched.
PERC_KEYS = [35, 36, 37, 38, 40, 41, 46, 47, 48, 49, 50, 51, 81]

# VCSL instruments: folder, and the file-name pattern giving (note, layer, round robin).
VCSL_PATCHES = {
    "lyre": {"dir": "Chordophones/Composite Chordophones/Folk Harp",
             "pattern": r"EWHarp_Normal_(?P<note>[A-G]#?-?\d)_v(?P<layer>\d)_RR(?P<rr>\d)\.wav"},
    "frame_drum": {"dir": "Membranophones/Struck Membranophones/Frame Drum",
                   "pattern": r"HDrum(?P<note>[LS])_Hit_v(?P<layer>\d)_rr(?P<rr>\d)_Sum\.wav"},
    "finger_cymbals": {"dir": "Idiophones/Struck Idiophones/Finger Cymbals",
                       "pattern": r"Fing_Cymb\.wav"},
}

# Colour instruments from FluidR3 (bank 0 program numbers).
FLUID_PATCHES = {
    "choir": 52,        # Choir Aahs
    "voices": 53,       # Voice Oohs (humming)
    "shawm": 111,
    "drone": 109,       # bagpipe chanter and drones: the hurdy-gurdy's buzz
    "synth": 80,        # Lead 1 (square), soft and filtered by the renderer
    "synth_bell": 98,   # FX 3 (crystal)
    "synth_pad": 89,    # Pad 2 (warm)
    "guitar": 29,       # Overdriven guitar
    "guitar_clean": 27,
    "solo_viola": 41,
    "solo_cello": 42,
    "viols": 42,
}
