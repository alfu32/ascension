-- bld.granary@automation: Future granary (mesh.granary_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.granary_automation','Future granary');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.granary_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.08,"steps":1}'),
  ('silo_1','include','mat.white_composite',0,'','{"mesh":"mesh.silo_future","x":-1.35,"y":0.08,"z":-1}'),
  ('silo_2','include','mat.white_composite',0,'','{"mesh":"mesh.silo_future","x":-0.45,"y":0.08,"z":-1}'),
  ('silo_3','include','mat.white_composite',0,'','{"mesh":"mesh.silo_future","x":0.45,"y":0.08,"z":-1}'),
  ('silo_4','include','mat.white_composite',0,'','{"mesh":"mesh.silo_future","x":1.35,"y":0.08,"z":-1}'),
  ('gallery','cylinder','mat.smart_glass',0,'','{"x":-1.6,"y":2.8,"z":-1,"w":0.3,"h":3.2,"segments":8,"roll":90}'),
  ('elevator','cylinder','mat.composite_panel',0,'','{"x":0.95,"y":0.08,"z":0.2,"w":0.75,"h":3.4,"segments":12}'),
  ('elevator_glass','cylinder','mat.smart_glass',0,'','{"x":0.95,"y":1.6,"z":0.2,"w":0.77,"h":0.5,"segments":12,"far":0}'),
  ('elevator_crown','cylinder','mat.white_composite',1,'','{"x":0.95,"y":3.48,"z":0.2,"w":0.8,"h":0.26,"segments":12}'),
  ('elevator_cap','dome','mat.white_composite',1,'','{"x":0.95,"y":3.74,"z":0.2,"w":0.78,"h":0.28,"segments":12}'),
  ('bridge','cylinder','mat.smart_glass',0,'','{"x":0.95,"y":3,"z":-0.65,"w":0.22,"h":0.6,"segments":6,"pitch":90,"far":0}'),
  ('chute','box','mat.white_composite',0,'','{"x":0.62,"y":1,"z":0.35,"w":0.08,"d":0.08,"h":0.9,"roll":-40,"far":0}'),
  ('pad','box','mat.graphite',0,'','{"x":0,"y":0.08,"z":0.95,"w":0.6,"d":1.2,"h":0.02,"far":0}'),
  ('hauler','box','mat.white_composite',0,'','{"x":0,"y":0.2,"z":0.95,"w":0.44,"d":0.9,"h":0.32,"taper":0.08}'),
  ('hauler_glow','box','mat.light_strip',0,'','{"x":0,"y":0.22,"z":1.41,"w":0.36,"d":0.02,"h":0.05,"far":0}'),
  ('hauler_wheels','cylinder','mat.rubber',0,'','{"x":-0.24,"y":0.18,"z":0.68,"w":0.24,"h":0.48,"segments":6,"roll":90,"repeat":2,"dz":0.55,"far":0}'),
  ('office','include','mat.white_composite',0,'','{"mesh":"mesh.house_future","x":-1.15,"y":0.08,"z":1,"scale":0.65}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.08,"z":1.65,"h":2.4,"fw":0.5,"fh":0.75,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.granary_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.granary'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.granary_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
