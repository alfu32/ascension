-- bld.smelter@industrial: Industrial smelter (mesh.smelter_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.smelter_industrial','Industrial smelter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.smelter_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('furnace_base','box','mat.red_brick',0,'','{"x":-0.55,"z":-0.6,"w":1.3,"d":1.3,"h":0.8}'),
  ('furnace','cylinder','mat.rust',0,'','{"x":-0.55,"y":0.8,"z":-0.6,"w":1.15,"h":2.3,"taper":0.35,"segments":10}'),
  ('furnace_bands','cylinder','mat.cast_iron',0,'','{"x":-0.55,"y":1.5,"z":-0.6,"w":1,"h":0.08,"segments":10,"far":0}'),
  ('furnace_top','cylinder','mat.cast_iron',0,'','{"x":-0.55,"y":3.1,"z":-0.6,"w":0.8,"h":0.35,"segments":8}'),
  ('furnace_glow','cylinder','mat.glow',0,'','{"x":-0.55,"y":3.45,"z":-0.6,"w":0.5,"h":0.04,"segments":8}'),
  ('downcomer','box','mat.cast_iron',0,'','{"x":0.1,"y":1.6,"z":-0.6,"w":0.14,"d":0.14,"h":1.9,"roll":-28}'),
  ('stoves','cylinder','mat.iron',0,'','{"x":0.55,"z":-1.2,"w":0.62,"h":2.6,"segments":8,"repeat":2,"dx":0.72,"tint":"#a6aab0"}'),
  ('stove_domes','dome','mat.iron',0,'','{"x":0.55,"y":2.6,"z":-1.2,"w":0.62,"h":0.3,"segments":8,"repeat":2,"dx":0.72,"tint":"#a6aab0"}'),
  ('stove_bands','cylinder','mat.cloth',1,'','{"x":0.55,"y":1.75,"z":-1.2,"w":0.64,"h":0.5,"segments":8,"repeat":2,"dx":0.72}'),
  ('chimney','cylinder','mat.red_brick',0,'','{"x":1.35,"y":0,"z":0.05,"w":0.55,"h":4.4,"taper":0.3,"segments":8}'),
  ('chimney_band','cylinder','mat.cloth',1,'','{"x":1.35,"y":3.344,"z":0.05,"w":0.462,"h":0.484,"segments":8}'),
  ('chimney_cap','cylinder','mat.cast_iron',0,'','{"x":1.35,"y":4.4,"z":0.05,"w":0.44,"h":0.12,"segments":8,"far":0}'),
  ('chimney_glow','cylinder','mat.glow',0,'','{"x":1.35,"y":4.5,"z":0.05,"w":0.303,"h":0.04,"segments":8,"far":0}'),
  ('cast_house','box','mat.red_brick',0,'','{"x":-0.55,"z":0.55,"w":1.5,"d":0.95,"h":0.95,"door":0.5,"door_h":0.6,"opening":"#3a2214"}'),
  ('cast_roof','gable_roof','mat.corrugated',0,'','{"x":-0.55,"y":0.95,"z":0.55,"w":1.5,"d":0.95,"pitch":26,"overhang":0.08,"t":0.04}'),
  ('cast_roof_cap','gable_roof','mat.cloth',1,'','{"x":-0.55,"y":1.11,"z":0.55,"w":1.66,"d":0.475,"pitch":26,"overhang":0,"t":0.03}'),
  ('tap_glow','box','mat.glow',0,'','{"x":-0.55,"y":0.05,"z":1,"w":0.48,"d":0.04,"h":0.55}'),
  ('ladle_rails_bed','box','mat.cinders',0,'','{"x":0.55,"y":0,"z":1.35,"w":2.4,"d":0.5,"h":0.05}'),
  ('ladle_rails_sleepers','box','mat.oak',0,'ladle_rails_bed','{"x":-1.05,"y":0.05,"w":0.08,"d":0.44,"h":0.02,"repeat":4,"dx":0.6,"far":0}'),
  ('ladle_rails','box','mat.cast_iron',0,'ladle_rails_bed','{"y":0.07,"z":-0.11,"w":2.4,"d":0.04,"h":0.04,"repeat":2,"dz":0.22,"far":0}'),
  ('ladle','cylinder','mat.rust',0,'','{"x":0.2,"y":0.2,"z":1.35,"w":0.45,"h":0.4,"segments":8}'),
  ('ladle_glow','cylinder','mat.glow',0,'','{"x":0.2,"y":0.6,"z":1.35,"w":0.5,"h":0.03,"segments":8,"far":0}'),
  ('ore','cone','mat.ore',0,'','{"x":-1.4,"z":-1.35,"w":0.9,"h":0.5,"segments":7}'),
  ('pigs','box','mat.cast_iron',0,'','{"x":1.1,"z":0.75,"w":0.6,"d":0.25,"h":0.08,"repeat":2,"dz":0.3,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.6,"z":1.6,"h":2.6,"fw":0.5,"fh":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.smelter_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.smelter'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.smelter_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
