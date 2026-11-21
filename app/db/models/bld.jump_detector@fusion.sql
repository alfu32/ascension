-- bld.jump_detector@fusion: Deep Space Tracking Station (mesh.jump_detector_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.jump_detector_fusion','Deep Space Tracking Station');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.jump_detector_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('pad','plinth','mat.concrete',0,'','{"w":1.85,"d":1.85,"h":0.18,"steps":2,"inset":0.1}'),
  ('base','box','mat.graphite',0,'','{"y":0.18,"w":0.95,"d":0.95,"h":0.32,"taper":0.18}'),
  ('base_band','box','mat.white_composite',1,'','{"y":0.42,"w":0.98,"d":0.98,"h":0.07,"far":0}'),
  ('mast','cylinder','mat.composite_panel',0,'','{"y":0.5,"w":0.3,"h":1.15,"taper":0.25,"segments":10}'),
  ('mast_glow','torus','mat.light_strip',0,'','{"y":0.95,"w":0.42,"t":0.04,"segments":14,"sides":4,"far":0}'),
  ('yoke','box','mat.white_composite',0,'','{"y":1.62,"w":0.5,"d":0.28,"h":0.22}'),
  ('dish','dome','mat.white_composite',0,'','{"y":2.02,"z":0.1,"w":1.55,"h":0.36,"segments":18,"pitch":145}'),
  ('dish_rim','torus','mat.white_composite',1,'','{"y":2.02,"z":0.1,"w":1.52,"t":0.05,"segments":18,"sides":4,"pitch":-35,"far":0}'),
  ('feed_arm','cylinder','mat.graphite',0,'','{"y":2.02,"z":0.1,"w":0.06,"h":0.75,"pitch":-35,"segments":5}'),
  ('feed','box','mat.light_strip',0,'','{"y":2.56,"z":-0.33,"w":0.12,"d":0.12,"h":0.12,"far":0}'),
  ('radome','dome','mat.white_composite',0,'','{"x":-0.6,"y":0.18,"z":-0.6,"w":0.5,"h":0.5,"segments":10}'),
  ('radome_band','cylinder','mat.light_strip',0,'','{"x":-0.6,"y":0.22,"z":-0.6,"w":0.52,"h":0.04,"segments":10,"far":0}'),
  ('console','box','mat.white_composite',0,'','{"x":0.58,"y":0.18,"z":-0.58,"w":0.42,"d":0.3,"h":0.3,"taper":0.2,"far":0}'),
  ('console_light','box','mat.signal_red',0,'','{"x":0.58,"y":0.48,"z":-0.58,"w":0.07,"d":0.07,"h":0.05,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.jump_detector_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.jump_detector'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.jump_detector_fusion'), (SELECT id FROM def WHERE key='mat.composite_panel'), 100);
