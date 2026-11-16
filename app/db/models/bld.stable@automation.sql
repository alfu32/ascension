-- bld.stable@automation: Future stable (mesh.stable_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.stable_automation','Future stable');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.stable_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.packed_earth',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('barn','box','mat.composite_panel',0,'','{"y":0.05,"z":-1,"w":3.4,"d":1.2,"h":0.65,"door":0.5,"door_h":0.55,"door_colour":"#3a4650","windows":6,"window_w":0.22,"window_h":0.18,"window_y":0.38,"opening":"#2c5258"}'),
  ('vault','cylinder','mat.white_composite',0,'','{"x":-1.7,"y":0.7,"z":-1,"w":1.2,"h":3.4,"segments":14,"roll":90}'),
  ('vault_stripe','box','mat.white_composite',1,'','{"y":1.24,"z":-1,"w":3.44,"d":0.42,"h":0.08}'),
  ('vault_glow','box','mat.light_strip',0,'','{"y":0.66,"z":-0.39,"w":3.42,"d":0.03,"h":0.03,"far":0}'),
  ('paddock','fence','mat.white_composite',0,'','{"z":0.75,"w":3.5,"d":1.9,"h":0.55,"spacing":0.85,"rails":3,"ring":1,"gap":0.8,"far":0}'),
  ('horse_w','include',NULL,0,'','{"mesh":"mesh.horse","x":-0.8,"z":0.75,"yaw":70,"scale":0.9}'),
  ('horse_e','include',NULL,0,'','{"mesh":"mesh.horse","x":0.7,"z":0.95,"yaw":-120,"scale":0.9}'),
  ('trough','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":0.15,"w":0.9,"d":0.28,"h":0.28,"far":0}'),
  ('bales','cylinder','mat.thatch',0,'','{"x":-1.6,"y":0.22,"z":0.2,"w":0.36,"h":0.32,"segments":8,"roll":90,"repeat":2,"dx":0.4}'),
  ('floodlight','cylinder','mat.white_composite',0,'','{"x":-1.55,"y":0.05,"z":-0.25,"w":0.05,"h":1.6,"segments":4,"far":0}'),
  ('floodlight_head','box','mat.light_strip',0,'','{"x":-1.55,"y":1.6,"z":-0.22,"w":0.2,"d":0.1,"h":0.08,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.55,"y":0.05,"z":1.7,"h":2.2,"fw":0.45,"fh":0.65,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.stable_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.stable'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.stable_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
