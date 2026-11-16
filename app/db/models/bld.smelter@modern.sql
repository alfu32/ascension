-- bld.smelter@modern: Modern smelter (mesh.smelter_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.smelter_modern','Modern smelter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.smelter_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('shop','box','mat.cladding',0,'','{"x":-0.55,"y":0.05,"z":-0.75,"w":2.3,"d":1.9,"h":1.9,"gable":10,"door":0.8,"door_h":1,"door_colour":"#2a2420","windows":1,"window_w":1.6,"window_h":0.18,"window_y":1.45,"side_windows":1,"opening":"#2b3b49","tint":"#a8b4bc"}'),
  ('tap_glow','box','mat.glow',0,'','{"x":-0.55,"y":0.05,"z":0.215,"w":0.6,"d":0.02,"h":0.6}'),
  ('shop_roof','gable_roof','mat.cladding',0,'','{"x":-0.55,"y":1.95,"z":-0.75,"w":2.3,"d":1.9,"pitch":10,"overhang":0.08,"t":0.05}'),
  ('shop_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":2.107,"z":-0.75,"w":2.5,"d":0.7,"pitch":10,"overhang":0,"t":0.03}'),
  ('electrodes','cylinder','mat.cast_iron',0,'','{"x":-0.85,"y":1.9,"z":-0.75,"w":0.09,"h":0.75,"segments":5,"repeat":3,"dx":0.3}'),
  ('duct','box','mat.white_metal',0,'','{"x":0.95,"y":1.65,"z":-0.8,"w":0.85,"d":0.22,"h":0.22}'),
  ('stack','cylinder','mat.concrete',0,'','{"x":1.35,"y":0.05,"z":-1.25,"w":0.5,"h":4.6,"taper":0.3,"segments":8}'),
  ('stack_band','cylinder','mat.cloth',1,'','{"x":1.35,"y":3.546,"z":-1.25,"w":0.408,"h":0.46,"segments":8}'),
  ('stack_cap','cylinder','mat.cast_iron',0,'','{"x":1.35,"y":4.65,"z":-1.25,"w":0.364,"h":0.1,"segments":8,"far":0}'),
  ('stack_light','box','mat.signal_red',0,'','{"x":1.35,"y":4.75,"z":-1.25,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('stack_2','cylinder','mat.concrete',0,'','{"x":1.35,"y":0.05,"z":-0.35,"w":0.42,"h":3.8,"taper":0.3,"segments":8}'),
  ('stack_2_band','cylinder','mat.cloth',1,'','{"x":1.35,"y":2.938,"z":-0.35,"w":0.341,"h":0.38,"segments":8}'),
  ('stack_2_cap','cylinder','mat.cast_iron',0,'','{"x":1.35,"y":3.85,"z":-0.35,"w":0.306,"h":0.1,"segments":8,"far":0}'),
  ('ladle_rails_bed','box','mat.cinders',0,'','{"x":0.3,"y":0,"z":1.35,"w":2.8,"d":0.5,"h":0.06}'),
  ('ladle_rails','box','mat.steel',0,'ladle_rails_bed','{"y":0.06,"z":-0.11,"w":2.8,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('ladle_car','box','mat.cast_iron',0,'','{"x":-0.3,"y":0.1,"z":1.35,"w":0.7,"d":0.45,"h":0.12}'),
  ('ladle','cylinder','mat.rust',0,'','{"x":-0.3,"y":0.22,"z":1.35,"w":0.5,"h":0.42,"taper":0.15,"segments":8}'),
  ('ladle_glow','cylinder','mat.glow',0,'','{"x":-0.3,"y":0.64,"z":1.35,"w":0.44,"h":0.03,"segments":8,"far":0}'),
  ('scrap','cone','mat.rust',0,'','{"x":1.15,"y":0.05,"z":0.55,"w":1,"h":0.5,"segments":7}'),
  ('magnet_crane','box','mat.white_metal',0,'','{"x":0.95,"y":0.05,"z":0.1,"w":0.1,"d":0.1,"h":1.5,"tint":"#e0a828","far":0}'),
  ('magnet_jib','box','mat.white_metal',0,'','{"x":1.15,"y":1.5,"z":0.1,"w":0.7,"d":0.1,"h":0.1,"tint":"#e0a828","far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.05,"z":1,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.smelter_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.smelter'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.smelter_modern'), (SELECT id FROM def WHERE key='mat.cladding'), 100);
