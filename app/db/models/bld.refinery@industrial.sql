-- bld.refinery@industrial: Refinery (mesh.refinery_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.refinery_industrial','Refinery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.refinery_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('column_a','cylinder','mat.iron',0,'','{"x":-1.1,"z":-1.15,"w":0.5,"h":3.3,"segments":8,"tint":"#c4c8cc"}'),
  ('column_a_bands','cylinder','mat.cloth',1,'','{"x":-1.1,"y":2.2,"z":-1.15,"w":0.52,"h":0.5,"segments":8}'),
  ('column_a_top','dome','mat.iron',0,'','{"x":-1.1,"y":3.3,"z":-1.15,"w":0.5,"h":0.18,"segments":8,"tint":"#c4c8cc"}'),
  ('column_b','cylinder','mat.iron',0,'','{"x":-0.35,"z":-1.25,"w":0.4,"h":2.6,"segments":8,"tint":"#c4c8cc"}'),
  ('column_b_bands','cylinder','mat.cloth',1,'','{"x":-0.35,"y":1.75,"z":-1.25,"w":0.42,"h":0.45,"segments":8}'),
  ('column_b_top','dome','mat.iron',0,'','{"x":-0.35,"y":2.6,"z":-1.25,"w":0.4,"h":0.15,"segments":8,"tint":"#c4c8cc"}'),
  ('platforms','cylinder','mat.cast_iron',0,'','{"x":-1.1,"y":1.2,"z":-1.15,"w":0.66,"h":0.04,"segments":8,"far":0}'),
  ('ladder','box','mat.cast_iron',0,'','{"x":-0.82,"z":-0.9,"w":0.08,"d":0.04,"h":3.2,"far":0}'),
  ('stills','box','mat.red_brick',0,'','{"x":-0.75,"z":-0.25,"w":1.4,"d":0.6,"h":0.55}'),
  ('still_fires','box','mat.glow',0,'','{"x":-1.15,"y":0.06,"z":0.06,"w":0.18,"d":0.02,"h":0.16,"repeat":3,"dx":0.4,"far":0}'),
  ('tank_1','cylinder','mat.rust',0,'','{"x":0.75,"z":-1,"w":1.05,"h":0.85,"segments":10}'),
  ('tank_1_roof','cone','mat.cast_iron',0,'','{"x":0.75,"y":0.85,"z":-1,"w":1.08,"h":0.16,"segments":10}'),
  ('tank_1_band','cylinder','mat.cloth',1,'','{"x":0.75,"y":0.5,"z":-1,"w":1.07,"h":0.3,"segments":10}'),
  ('tank_2','cylinder','mat.iron',0,'','{"x":1.1,"z":0.3,"w":0.9,"h":0.75,"segments":10,"tint":"#b4b8bc"}'),
  ('tank_2_band','cylinder','mat.cloth',1,'','{"x":1.1,"y":0.5,"z":0.3,"w":0.92,"h":0.2,"segments":10}'),
  ('tank_2_roof','cone','mat.cast_iron',0,'','{"x":1.1,"y":0.75,"z":0.3,"w":0.92,"h":0.14,"segments":10}'),
  ('tank_3','cylinder','mat.iron',0,'','{"x":0.25,"z":0.45,"w":0.7,"h":0.6,"segments":9,"tint":"#b4b8bc"}'),
  ('tank_3_band','cylinder','mat.cloth',1,'','{"x":0.25,"y":0.38,"z":0.45,"w":0.72,"h":0.16,"segments":9}'),
  ('tank_3_roof','cone','mat.cast_iron',0,'','{"x":0.25,"y":0.6,"z":0.45,"w":0.72,"h":0.12,"segments":9,"far":0}'),
  ('pipe_rack','box','mat.cast_iron',0,'','{"x":0,"y":1,"z":-0.4,"w":2.6,"d":0.08,"h":0.08}'),
  ('pipe_rack_2','box','mat.cast_iron',0,'','{"x":0,"y":0.9,"z":-0.5,"w":2.6,"d":0.06,"h":0.06,"far":0}'),
  ('rack_posts','box','mat.cast_iron',0,'','{"x":-1.2,"z":-0.45,"w":0.06,"d":0.14,"h":1,"repeat":4,"dx":0.8,"far":0}'),
  ('pipe_down','box','mat.cast_iron',0,'','{"x":1.25,"y":0,"z":-0.45,"w":0.07,"d":0.07,"h":1,"far":0}'),
  ('flare_stack','cylinder','mat.cast_iron',0,'','{"x":-1.45,"z":1.35,"w":0.16,"h":3,"segments":6}'),
  ('flare_stays','box','mat.steel',0,'','{"x":-1.45,"z":1.35,"w":0.02,"d":0.02,"h":2.4,"roll":14,"far":0}'),
  ('flare','cone','mat.glow',0,'','{"x":-1.45,"y":3,"z":1.35,"w":0.26,"h":0.5,"segments":6}'),
  ('office','box','mat.red_brick',0,'','{"x":-0.65,"z":1.05,"w":1,"d":0.7,"h":0.85,"door":0.25,"door_h":0.5,"windows":2,"window_y":0.32,"window_w":0.14,"window_h":0.28,"opening":"#22282f"}'),
  ('office_roof','gable_roof','mat.slate',0,'','{"x":-0.65,"y":0.85,"z":1.05,"w":1,"d":0.7,"pitch":30,"overhang":0.06,"t":0.05}'),
  ('office_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.65,"y":1.009,"z":1.05,"w":1.12,"d":0.35,"pitch":30,"overhang":0,"t":0.03}'),
  ('barrels','cylinder','mat.oak',0,'','{"x":0.55,"z":1.4,"w":0.22,"h":0.3,"segments":6,"repeat":3,"dx":0.26,"far":0,"tint":"#7a5a40"}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"z":1.6,"h":2.4,"fw":0.5,"fh":0.75}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.refinery_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.refinery'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.refinery_industrial'), (SELECT id FROM def WHERE key='mat.iron'), 100);
