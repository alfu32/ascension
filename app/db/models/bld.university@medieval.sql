-- bld.university@medieval: Medieval university (mesh.university_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.university_medieval','Medieval university');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.university_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','plinth','mat.rough_stone',0,'','{"w":3.7,"d":3.7,"h":0.18,"steps":1}'),
  ('hall','box','mat.dressed_stone',0,'','{"y":0.18,"z":-1.2,"w":3.5,"d":1.1,"h":1.9,"gable":50,"windows":5,"window_y":0.55,"window_w":0.14,"window_h":0.6,"side_windows":1}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"y":2.08,"z":-1.2,"w":3.5,"d":1.1,"pitch":50,"overhang":0.1,"t":0.08}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"y":2.622,"z":-1.2,"w":3.74,"d":0.4,"pitch":50,"overhang":0,"t":0.03}'),
  ('wing_w','box','mat.rough_stone',0,'','{"x":-1.35,"y":0.18,"z":0.3,"yaw":90,"w":1.9,"d":0.9,"h":1.5,"gable":50,"windows":3,"window_y":0.55,"window_w":0.14,"window_h":0.36}'),
  ('wing_e','box','mat.rough_stone',0,'','{"x":1.35,"y":0.18,"z":0.3,"yaw":-90,"w":1.9,"d":0.9,"h":1.5,"gable":50,"windows":3,"window_y":0.55,"window_w":0.14,"window_h":0.36}'),
  ('wing_roof_w','gable_roof','mat.slate',0,'','{"x":-1.35,"y":1.68,"z":0.3,"yaw":90,"w":1.9,"d":0.9,"pitch":50,"overhang":0.06,"t":0.08}'),
  ('wing_roof_w_cap','gable_roof','mat.cloth',1,'','{"x":-1.35,"y":2.126,"z":0.3,"yaw":90,"w":2.06,"d":0.36,"pitch":50,"overhang":0,"t":0.03}'),
  ('wing_roof_e','gable_roof','mat.slate',0,'','{"x":1.35,"y":1.68,"z":0.3,"yaw":90,"w":1.9,"d":0.9,"pitch":50,"overhang":0.06,"t":0.08}'),
  ('wing_roof_e_cap','gable_roof','mat.cloth',1,'','{"x":1.35,"y":2.126,"z":0.3,"yaw":90,"w":2.06,"d":0.36,"pitch":50,"overhang":0,"t":0.03}'),
  ('gate_tower','box','mat.dressed_stone',0,'','{"y":0.18,"z":1.15,"w":1.25,"d":1,"h":2.7,"door":0.62,"door_h":0.95,"door_colour":"#2a221c","windows":1,"window_y":1.95,"window_w":0.16,"window_h":0.4,"side_windows":1}'),
  ('gate','arch','mat.dressed_stone',0,'','{"y":0.18,"z":1.66,"w":1.25,"h":1.3,"t":0.06,"span":0.6,"spring":0.6,"tint":"#d8d2c8"}'),
  ('gate_top','box','mat.dressed_stone',0,'','{"y":2.88,"z":1.15,"w":1.39,"d":1.14,"h":0.16,"tint":"#d8d2c8"}'),
  ('gate_merlons','crenellation','mat.dressed_stone',0,'','{"y":3.04,"z":1.15,"w":1.39,"d":1.14,"h":0.28,"t":0.14,"merlon":0.22,"ring":1}'),
  ('gate_cloth','box','mat.cloth',1,'','{"y":1.65,"z":1.66,"w":0.7,"d":0.02,"h":0.75}'),
  ('banners','banner','mat.cloth',1,'','{"x":-0.95,"y":0.18,"z":1.4,"h":2.6,"fw":0.42,"fh":0.8,"repeat":2,"dx":1.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.university_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.university'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.university_medieval'), (SELECT id FROM def WHERE key='mat.dressed_stone'), 100);
