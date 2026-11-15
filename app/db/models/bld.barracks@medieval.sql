-- bld.barracks@medieval: Medieval barracks (mesh.barracks_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_medieval','Medieval barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('walls','wall','mat.rough_stone',0,'','{"w":3.5,"d":3.5,"h":1.15,"t":0.22,"ring":1,"gap":1.3}'),
  ('merlons','crenellation','mat.rough_stone',0,'','{"y":1.15,"w":3.5,"d":3.5,"h":0.26,"t":0.22,"merlon":0.3,"ring":1,"gap":1.3,"far":0}'),
  ('gate','arch','mat.dressed_stone',0,'','{"z":1.64,"w":1.3,"h":1.45,"t":0.3,"span":0.7,"spring":0.7}'),
  ('gate_cloth','box','mat.cloth',1,'','{"y":1.06,"z":1.8,"w":0.6,"d":0.02,"h":0.3}'),
  ('tower_w','include',NULL,0,'','{"mesh":"mesh.round_tower","x":-0.82,"z":1.62,"scale":0.62}'),
  ('tower_e','include',NULL,0,'','{"mesh":"mesh.round_tower","x":0.82,"z":1.62,"scale":0.62}'),
  ('hall','box','mat.rough_stone',0,'','{"y":0.05,"z":-0.95,"w":2.7,"d":1.2,"h":1.3,"gable":46,"door":0.45,"door_h":0.85,"windows":3,"window_y":0.65,"window_w":0.14,"window_h":0.3}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"y":1.35,"z":-0.95,"w":2.7,"d":1.2,"pitch":46,"overhang":0.14,"t":0.08}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"y":1.828,"z":-0.95,"w":3.02,"d":0.5,"pitch":46,"overhang":0,"t":0.03}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.15,"z":0.45,"h":3.9,"fw":0.8,"fh":1.35,"notch":0.25}'),
  ('rack','fence','mat.oak',0,'','{"x":-0.9,"z":-0.05,"w":1,"h":0.55,"spacing":0.5,"rails":1,"far":0}'),
  ('pikes','cylinder','mat.timber',0,'','{"x":-1.3,"z":-0.1,"w":0.04,"h":2,"segments":4,"repeat":4,"dx":0.25,"far":0}'),
  ('target','cylinder','mat.thatch',0,'','{"x":0.2,"y":0.55,"z":0.2,"w":0.55,"h":0.12,"segments":8,"pitch":80}'),
  ('target_face','cylinder','mat.cloth',1,'','{"x":0.2,"y":0.56,"z":0.32,"w":0.3,"h":0.03,"segments":8,"pitch":80,"far":0}'),
  ('target_legs','box','mat.oak',0,'','{"x":0,"z":0.05,"w":0.05,"d":0.05,"h":0.75,"repeat":2,"dx":0.4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.barracks_medieval'), (SELECT id FROM def WHERE key='mat.rough_stone'), 100);
