-- bld.market@automation: Future market (mesh.market_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_automation','Future market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.06,"steps":1}'),
  ('store','box','mat.composite_panel',0,'','{"y":0.06,"z":-0.95,"w":3.4,"d":1.5,"h":1,"door":0.6,"door_h":0.7,"door_colour":"#2c3a40","windows":4,"window_w":0.6,"window_h":0.6,"window_y":0.15,"opening":"#2c5258"}'),
  ('store_vault','cylinder','mat.white_composite',0,'','{"x":-1.7,"y":1.06,"z":-0.95,"w":1.5,"h":3.4,"segments":14,"roll":90}'),
  ('store_glow','box','mat.light_strip',0,'','{"y":1.03,"z":-0.19,"w":3.42,"d":0.03,"h":0.03,"far":0}'),
  ('sign_band','box','mat.white_composite',1,'','{"y":0.78,"z":-0.18,"w":3.44,"d":0.06,"h":0.24}'),
  ('counter_w','box','mat.white_composite',0,'','{"x":-1.15,"y":0.06,"z":1,"yaw":12,"w":0.85,"d":0.36,"h":0.36}'),
  ('goods_w','box','mat.white_composite',0,'','{"x":-1.15,"y":0.42,"z":1,"yaw":12,"w":0.7,"d":0.26,"h":0.06,"tint":"#ffffff","far":0}'),
  ('awning_w','awning','mat.white_composite',1,'','{"x":-1.15,"y":0.06,"z":0.85,"yaw":12,"w":1.05,"d":0.75,"h":0.85,"rise":0.2,"posts":0}'),
  ('posts_w','cylinder','mat.white_composite',0,'','{"x":-1.62,"y":0.06,"z":1.18,"w":0.05,"h":0.85,"segments":4,"repeat":2,"dx":0.94,"far":0}'),
  ('counter_c','box','mat.white_composite',0,'','{"x":0,"y":0.06,"z":1.15,"yaw":0,"w":0.85,"d":0.36,"h":0.36}'),
  ('goods_c','box','mat.greens',0,'','{"x":0,"y":0.42,"z":1.15,"yaw":0,"w":0.7,"d":0.26,"h":0.06,"tint":"#ffffff","far":0}'),
  ('awning_c','awning','mat.white_composite',1,'','{"x":0,"y":0.06,"z":1,"yaw":0,"w":1.05,"d":0.75,"h":0.85,"rise":0.2,"posts":0}'),
  ('posts_c','cylinder','mat.white_composite',0,'','{"x":-0.47,"y":0.06,"z":1.33,"w":0.05,"h":0.85,"segments":4,"repeat":2,"dx":0.94,"far":0}'),
  ('counter_e','box','mat.white_composite',0,'','{"x":1.15,"y":0.06,"z":1,"yaw":-12,"w":0.85,"d":0.36,"h":0.36}'),
  ('goods_e','box','mat.white_composite',0,'','{"x":1.15,"y":0.42,"z":1,"yaw":-12,"w":0.7,"d":0.26,"h":0.06,"tint":"#f2b048","far":0}'),
  ('awning_e','awning','mat.white_composite',1,'','{"x":1.15,"y":0.06,"z":0.85,"yaw":-12,"w":1.05,"d":0.75,"h":0.85,"rise":0.2,"posts":0}'),
  ('posts_e','cylinder','mat.white_composite',0,'','{"x":0.68,"y":0.06,"z":1.18,"w":0.05,"h":0.85,"segments":4,"repeat":2,"dx":0.94,"far":0}'),
  ('pylon','cylinder','mat.white_composite',0,'','{"x":1.6,"y":0.06,"z":1.6,"w":0.12,"h":2,"segments":6}'),
  ('pylon_sign','box','mat.white_composite',1,'','{"x":1.6,"y":1.5,"z":1.6,"w":0.12,"d":0.6,"h":0.5}'),
  ('pylon_glow','box','mat.light_strip',0,'','{"x":1.6,"y":1.45,"z":1.6,"w":0.14,"d":0.62,"h":0.04,"far":0}'),
  ('pad','cylinder','mat.graphite',0,'','{"x":-1.45,"y":0.06,"z":1.5,"w":0.55,"h":0.03,"segments":10,"far":0}'),
  ('drone','box','mat.white_composite',0,'','{"x":-1.45,"y":0.09,"z":1.5,"w":0.26,"d":0.26,"h":0.1,"far":0}'),
  ('planters','cylinder','mat.white_composite',0,'','{"x":1.5,"y":0.06,"z":0.1,"w":0.36,"h":0.22,"segments":8,"far":0}'),
  ('shrub','dome','mat.greens',0,'','{"x":1.5,"y":0.28,"z":0.1,"w":0.36,"h":0.3,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.market_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
