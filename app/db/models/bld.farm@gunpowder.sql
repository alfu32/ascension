-- bld.farm@gunpowder: Early modern farm (mesh.farm_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.farm_gunpowder','Early modern farm');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.farm_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('soil','plinth','mat.soil',0,'','{"w":3.8,"d":3.8,"h":0.03,"steps":1}'),
  ('barn','box','mat.brick',0,'','{"x":-1.15,"z":-1.25,"w":1.25,"d":0.95,"h":0.78,"gable":44,"door":0.44,"door_h":0.6,"door_colour":"#3a2a1c","side_windows":1,"window_y":0.36,"window_w":0.12,"window_h":0.22}'),
  ('quoins','box','mat.ashlar',0,'','{"x":-1.76,"z":-1.72,"w":0.08,"d":0.08,"h":0.78,"repeat":2,"dx":1.22,"far":0}'),
  ('quoins_s','box','mat.ashlar',0,'','{"x":-1.76,"z":-0.78,"w":0.08,"d":0.08,"h":0.78,"repeat":2,"dx":1.22,"far":0}'),
  ('roof','gable_roof','mat.pantile',0,'','{"x":-1.15,"y":0.78,"z":-1.25,"w":1.25,"d":0.95,"pitch":44,"overhang":0.12,"t":0.08}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"x":-1.15,"y":1.137,"z":-1.25,"w":1.5,"d":0.44,"pitch":44,"overhang":0,"t":0.03}'),
  ('dovecote','box','mat.lime_plaster',0,'','{"x":-1.15,"y":1.15,"z":-1.25,"w":0.22,"d":0.22,"h":0.22,"far":0}'),
  ('dovecote_roof','hip_roof','mat.pantile',0,'','{"x":-1.15,"y":1.37,"z":-1.25,"w":0.22,"d":0.22,"pitch":50,"overhang":0.05,"t":0.03,"far":0}'),
  ('rick','cone','mat.thatch',0,'','{"x":-0.25,"z":-1.45,"w":0.62,"h":0.78,"segments":7}'),
  ('rick_2','cone','mat.thatch',0,'','{"x":0.42,"z":-1.5,"w":0.5,"h":0.62,"segments":7,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.5,"z":-0.7,"h":1.85,"fw":0.42,"fh":0.55}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.farm_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.farm'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.farm_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
