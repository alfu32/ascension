-- bld.barracks@classical: Classical barracks (mesh.barracks_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_classical','Classical barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.limestone',0,'','{"w":3.6,"d":3.6,"h":1.15,"t":0.2,"ring":1,"gap":1.3}'),
  ('gate','arch','mat.limestone',0,'','{"z":1.7,"w":1.7,"h":1.75,"t":0.26,"span":0.85,"spring":0.85}'),
  ('gate_cloth','box','mat.cloth',1,'','{"y":1.32,"z":1.835,"w":0.75,"d":0.02,"h":0.36}'),
  ('block','box','mat.lime_plaster',0,'','{"y":0.05,"z":-0.95,"w":2.7,"d":1.2,"h":1.35,"gable":22,"door":0.45,"door_h":0.9,"windows":4,"window_y":0.7,"window_w":0.18,"window_h":0.28,"side_windows":1}'),
  ('roof','gable_roof','mat.terracotta',0,'','{"y":1.4,"z":-0.95,"w":2.7,"d":1.2,"pitch":22,"overhang":0.15,"t":0.08}'),
  ('ridge_cap','gable_roof','mat.cloth',1,'','{"y":1.587,"z":-0.95,"w":3,"d":0.7,"pitch":22,"overhang":0,"t":0.03}'),
  ('ridge','box','mat.cloth',1,'','{"y":1.729,"z":-0.95,"w":2.95,"d":0.16,"h":0.08}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.15,"z":0.6,"h":3.8,"fw":0.8,"fh":1.35,"notch":0.25}'),
  ('shields','cylinder','mat.cloth',1,'','{"x":-1.05,"y":0.19,"z":-0.22,"w":0.38,"h":0.05,"segments":8,"pitch":-80,"repeat":3,"dx":0.34,"far":0}'),
  ('rack','fence','mat.timber',0,'','{"x":0.95,"z":-0.15,"w":0.9,"h":0.55,"spacing":0.45,"rails":1,"far":0}'),
  ('spears','cylinder','mat.timber',0,'','{"x":0.65,"z":-0.2,"w":0.045,"h":1.5,"segments":4,"repeat":4,"dx":0.2,"far":0}'),
  ('posts','cylinder','mat.timber',0,'','{"x":-1.0,"z":0.8,"w":0.13,"h":1.2,"segments":6,"repeat":2,"dx":0.8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.barracks_classical'), (SELECT id FROM def WHERE key='mat.lime_plaster'), 100);
