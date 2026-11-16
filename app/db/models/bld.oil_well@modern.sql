-- bld.oil_well@modern: Modern oil well (mesh.oil_well_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.oil_well_modern','Modern oil well');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.oil_well_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":1.8,"d":1.8,"h":0.04,"steps":1,"tint":"#8a7a68"}'),
  ('pool','cylinder','mat.crude',0,'','{"x":-0.45,"y":0.04,"z":0.5,"w":0.5,"h":0.01,"segments":8,"far":0}'),
  ('skid','box','mat.steel',0,'','{"x":-0.05,"y":0.04,"z":-0.2,"w":1.35,"d":0.4,"h":0.08}'),
  ('samson','box','mat.steel',0,'','{"x":0,"y":0.12,"z":-0.38,"w":0.08,"d":0.08,"h":0.9,"pitch":12}'),
  ('samson_s','box','mat.steel',0,'','{"x":0,"y":0.12,"z":-0.02,"w":0.08,"d":0.08,"h":0.9,"pitch":-12}'),
  ('beam','box','mat.cloth',1,'','{"x":-0.15,"y":0.98,"z":-0.2,"w":1.25,"d":0.12,"h":0.13,"roll":5}'),
  ('head','box','mat.steel',0,'','{"x":-0.78,"y":0.7,"z":-0.2,"w":0.14,"d":0.14,"h":0.42,"roll":-12}'),
  ('rod','cylinder','mat.steel',0,'','{"x":-0.83,"y":0.12,"z":-0.2,"w":0.03,"h":0.6,"segments":4}'),
  ('wellhead','cylinder','mat.steel',0,'','{"x":-0.83,"y":0.04,"z":-0.2,"w":0.14,"h":0.16,"segments":6}'),
  ('valves','box','mat.white_metal',0,'','{"x":-0.83,"y":0.08,"z":-0.2,"w":0.06,"d":0.3,"h":0.06,"tint":"#c04838","far":0}'),
  ('gearbox','box','mat.cast_iron',0,'','{"x":0.45,"y":0.12,"z":-0.2,"w":0.28,"d":0.22,"h":0.28}'),
  ('cranks','shield','mat.cast_iron',0,'','{"x":0.45,"y":0.42,"z":-0.4,"w":0.5,"t":0.05,"boss":0.04,"segments":8,"rim":"#3a3e44","repeat":2,"dz":0.4}'),
  ('pitmans','box','mat.steel',0,'','{"x":0.42,"y":0.45,"z":-0.42,"w":0.04,"d":0.04,"h":0.55,"repeat":2,"dz":0.44,"far":0}'),
  ('motor','box','mat.white_metal',0,'','{"x":0.73,"y":0.12,"z":-0.2,"w":0.2,"d":0.2,"h":0.16,"far":0}'),
  ('tank','cylinder','mat.white_metal',0,'','{"x":0.52,"y":0.04,"z":0.52,"w":0.42,"h":0.5,"segments":9}'),
  ('tank_band','cylinder','mat.cloth',1,'','{"x":0.52,"y":0.3,"z":0.52,"w":0.44,"h":0.12,"segments":9}'),
  ('tank_2','cylinder','mat.white_metal',0,'','{"x":0.06,"y":0.04,"z":0.58,"w":0.36,"h":0.42,"segments":9}'),
  ('pipe','box','mat.steel',0,'','{"x":-0.4,"y":0.05,"z":0.15,"w":0.05,"d":0.05,"h":0.05,"far":0}'),
  ('pipe_run','box','mat.steel',0,'','{"x":0.1,"y":0.05,"z":0.18,"w":1,"d":0.05,"h":0.05,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.oil_well_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.oil_well'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.oil_well_modern'), (SELECT id FROM def WHERE key='mat.steel'), 100);
