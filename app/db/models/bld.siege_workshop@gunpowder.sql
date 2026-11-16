-- bld.siege_workshop@gunpowder: Early modern siege workshop (mesh.siege_workshop_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.siege_workshop_gunpowder','Early modern siege workshop');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.siege_workshop_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('piers_back','box','mat.brick',0,'','{"x":-1.55,"z":-1.6,"w":0.2,"d":0.2,"h":2,"repeat":3,"dx":1.55}'),
  ('piers_front','box','mat.brick',0,'','{"x":-1.55,"z":-0.55,"w":0.2,"d":0.2,"h":2,"repeat":3,"dx":1.55}'),
  ('back_wall','wall','mat.brick',0,'','{"z":-1.62,"w":3.1,"h":1.3,"t":0.12}'),
  ('roof','gable_roof','mat.pantile',0,'','{"y":2,"z":-1.075,"w":3.3,"d":1.25,"pitch":40,"overhang":0.18,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"y":2.419,"z":-1.075,"w":3.7,"d":0.5,"pitch":40,"overhang":0,"t":0.03}'),
  ('hanging','box','mat.cloth',1,'','{"x":0.78,"y":1,"z":-1.54,"w":1.1,"d":0.03,"h":0.85}'),
  ('trestles','box','mat.oak',0,'','{"x":-0.8,"z":-1.05,"w":0.1,"d":0.6,"h":0.5,"repeat":2,"dx":0.9,"far":0}'),
  ('barrel_bare','cylinder','mat.bronze',0,'','{"x":-1.15,"y":0.61,"z":-1.05,"w":0.22,"h":1.1,"taper":0.3,"segments":8,"roll":90}'),
  ('bench','box','mat.oak',0,'','{"x":0.8,"z":-1.2,"w":1.2,"d":0.5,"h":0.55}'),
  ('gun_wheels','cylinder','mat.oak',0,'','{"x":-0.35,"y":0.36,"z":0.7,"w":0.72,"h":0.07,"segments":10,"roll":90,"repeat":2,"dx":0.7}'),
  ('gun_cheeks','box','mat.oak',0,'','{"x":0,"y":0.25,"z":0.55,"w":0.36,"d":0.9,"h":0.22}'),
  ('gun_trail','box','mat.oak',0,'','{"x":0,"z":-0.15,"w":0.2,"d":0.1,"h":0.85,"pitch":62}'),
  ('gun_barrel','cylinder','mat.bronze',0,'','{"x":0,"y":0.5,"z":0.35,"w":0.2,"h":1,"taper":0.3,"segments":8,"pitch":86}'),
  ('shot','dome','mat.iron',0,'','{"x":1.05,"z":0.75,"w":0.5,"h":0.3,"segments":6,"tint":"#5a5e66"}'),
  ('spare_wheel','cylinder','mat.oak',0,'','{"x":1.15,"z":1.25,"w":0.6,"h":0.08,"segments":10,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.55,"z":1.55,"h":2.8,"fw":0.55,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.siege_workshop_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.siege_workshop'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.siege_workshop_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
