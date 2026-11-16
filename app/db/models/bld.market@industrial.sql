-- bld.market@industrial: Industrial market (mesh.market_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_industrial','Industrial market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.dressed_stone',0,'','{"w":3.7,"d":3.7,"h":0.06,"steps":1,"tint":"#b0aa9e"}'),
  ('back_wall','wall','mat.red_brick',0,'','{"y":0.06,"z":-1.45,"w":2.6,"h":1.6,"t":0.18,"door":0.6,"door_h":0.9,"opening":"#22282f"}'),
  ('back_gable','box','mat.red_brick',0,'','{"y":1.66,"z":-1.45,"yaw":90,"w":0.18,"d":2.6,"h":0.02,"gable":32}'),
  ('back_window','shield','mat.glass',0,'','{"y":2,"z":-1.35,"w":0.56,"t":0.02,"boss":0,"segments":10,"rim":"#3a3e44","far":0}'),
  ('columns_w','column_row','mat.cast_iron',0,'','{"x":-1.15,"y":0.06,"z":-0.3,"yaw":90,"count":3,"radius":0.05,"spacing":1,"h":1.6,"segments":6,"capital":1}'),
  ('columns_e','column_row','mat.cast_iron',0,'','{"x":1.15,"y":0.06,"z":-0.3,"yaw":90,"count":3,"radius":0.05,"spacing":1,"h":1.6,"segments":6,"capital":1}'),
  ('girders','box','mat.cast_iron',0,'','{"x":-1.15,"y":1.62,"z":-0.3,"w":0.08,"d":2.4,"h":0.1,"repeat":2,"dx":2.3}'),
  ('roof','gable_roof','mat.glass',0,'','{"yaw":90,"y":1.7,"z":-0.3,"w":2.5,"d":2.6,"pitch":32,"overhang":0.12,"t":0.05}'),
  ('roof_cap','gable_roof','mat.cloth',1,'','{"yaw":90,"y":2.353,"z":-0.3,"w":2.74,"d":0.7,"pitch":32,"overhang":0,"t":0.03}'),
  ('front_gable','box','mat.glass',0,'','{"y":1.66,"z":0.95,"yaw":90,"w":0.04,"d":2.6,"h":0.02,"gable":32,"far":0}'),
  ('lantern','box','mat.cast_iron',0,'','{"y":2.5,"z":-0.3,"w":0.26,"d":1.2,"h":0.18,"far":0}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-0.55,"y":0.06,"z":-0.75,"yaw":90,"scale":0.7}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":0.55,"y":0.06,"z":-0.75,"yaw":-90,"scale":0.7}'),
  ('stall_s','include',NULL,0,'','{"mesh":"mesh.stall","x":-0.9,"y":0.06,"z":1.4,"scale":0.8}'),
  ('stall_se','include',NULL,0,'','{"mesh":"mesh.stall","x":0.95,"y":0.06,"z":1.4,"scale":0.8}'),
  ('clock','shield','mat.canvas',0,'','{"y":1.3,"z":-1.34,"w":0.34,"t":0.03,"boss":0.04,"segments":10,"rim":"#2a2622","far":0}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":-1.6,"y":0.06,"z":0.3,"w":0.26,"h":0.36,"segments":7,"repeat":2,"dz":0.32,"tint":"#c8a888","far":0}'),
  ('crates','box','mat.timber',0,'','{"x":1.55,"y":0.06,"z":0.4,"w":0.3,"d":0.3,"h":0.3,"repeat":2,"dz":0.35,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.market_industrial'), (SELECT id FROM def WHERE key='mat.timber'), 100);
