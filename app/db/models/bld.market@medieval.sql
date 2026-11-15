-- bld.market@medieval: Medieval market (mesh.market_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_medieval','Medieval market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.rough_stone',0,'','{"w":3.7,"d":3.7,"h":0.06,"steps":1,"tint":"#c4beb2"}'),
  ('hall_posts','column_row','mat.oak',0,'','{"y":0.06,"z":-0.6,"count":4,"radius":0.08,"spacing":0.8,"h":1,"segments":6,"capital":0}'),
  ('hall_posts_back','column_row','mat.oak',0,'','{"y":0.06,"z":-1.5,"count":4,"radius":0.08,"spacing":0.8,"h":1,"segments":6,"capital":0,"far":0}'),
  ('hall_sill','box','mat.oak',0,'','{"y":1.02,"z":-1.05,"w":2.7,"d":1.3,"h":0.1}'),
  ('hall_upper','box','mat.daub',0,'','{"y":1.12,"z":-1.05,"w":2.6,"d":1.2,"h":0.72,"gable":50,"windows":3,"window_y":0.24,"window_w":0.16,"window_h":0.24}'),
  ('hall_frame','box','mat.oak',0,'','{"x":-1.3,"y":1.12,"z":-1.05,"w":0.06,"d":1.23,"h":0.72,"repeat":5,"dx":0.65,"far":0}'),
  ('hall_roof','gable_roof','mat.slate',0,'','{"y":1.84,"z":-1.05,"w":2.6,"d":1.2,"pitch":50,"overhang":0.14,"t":0.08}'),
  ('hall_roof_cap','gable_roof','mat.cloth',1,'','{"y":2.382,"z":-1.05,"w":2.92,"d":0.5,"pitch":50,"overhang":0,"t":0.03}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.3,"y":0.06,"z":0.6,"yaw":90,"scale":0.85}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.3,"y":0.06,"z":0.6,"yaw":-90,"scale":0.85}'),
  ('stall_s','include',NULL,0,'','{"mesh":"mesh.stall","y":0.06,"z":1.3,"yaw":180,"scale":0.85}'),
  ('column_base','plinth','mat.dressed_stone',0,'','{"y":0.06,"z":0.35,"w":0.6,"d":0.6,"h":0.24,"steps":2,"inset":0.08}'),
  ('column','cylinder','mat.dressed_stone',0,'','{"y":0.3,"z":0.35,"w":0.16,"h":1.2,"segments":6}'),
  ('column_top','dome','mat.cloth',1,'','{"y":1.48,"z":0.35,"w":0.26,"h":0.2,"segments":6}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":-0.55,"y":0.06,"z":-0.25,"w":0.26,"h":0.36,"segments":7,"repeat":3,"dx":0.32,"tint":"#c8a888"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.market_medieval'), (SELECT id FROM def WHERE key='mat.daub'), 100);
