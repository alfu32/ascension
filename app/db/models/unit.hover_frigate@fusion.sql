-- unit.hover_frigate@fusion: Hover frigate (mesh.hover_frigate_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.hover_frigate_fusion','Hover frigate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.hover_frigate_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','hull','mat.white_composite',0,'','{"yaw":-90,"y":0.34,"w":3.6,"d":0.8,"h":0.45,"sheer":0.12,"segments":10,"bank":4}'),
  ('deck','box','mat.white_composite',0,'hull','{"y":0.4,"w":2.7,"d":0.74,"h":0.04}'),
  ('pods','cylinder','mat.white_composite',0,'hull','{"x":-1.25,"y":0.02,"z":-0.62,"w":0.26,"h":2.5,"segments":6,"roll":90,"repeat":2,"dz":1.24}'),
  ('pod_noses','cone','mat.white_composite',0,'hull','{"x":1.25,"y":0.02,"z":-0.62,"w":0.26,"h":0.4,"segments":6,"roll":90,"repeat":2,"dz":1.24,"far":0}'),
  ('pod_bands','box','mat.white_composite',1,'hull','{"x":-0.9,"y":0.13,"z":-0.62,"w":1.9,"d":0.2,"h":0.04,"repeat":2,"dz":1.24}'),
  ('pod_glow','box','mat.plasma',0,'hull','{"x":-1.1,"y":-0.13,"z":-0.62,"w":2.3,"d":0.16,"h":0.03,"repeat":2,"dz":1.24}'),
  ('struts','box','mat.graphite',0,'hull','{"x":-0.7,"y":0.08,"w":0.18,"d":1.3,"h":0.07,"repeat":2,"dx":1.2,"far":0}'),
  ('superstructure','box','mat.white_composite',0,'hull','{"x":-0.15,"y":0.44,"w":1.5,"d":0.6,"h":0.26,"taper":0.15}'),
  ('bridge','box','mat.smart_glass',0,'hull','{"x":0.2,"y":0.6,"w":0.8,"d":0.48,"h":0.1,"taper":0.1,"far":0}'),
  ('radome_ring','cylinder','mat.white_composite',1,'hull','{"x":-0.45,"y":0.7,"w":0.5,"h":0.08,"segments":8}'),
  ('radome','dome','mat.white_composite',0,'hull','{"x":-0.45,"y":0.78,"w":0.5,"h":0.34,"segments":8}'),
  ('cells','box','mat.graphite',0,'hull','{"x":-1.1,"y":0.44,"w":0.4,"d":0.44,"h":0.04,"far":0}'),
  ('drives','wheel','mat.plasma',0,'hull','{"x":-1.82,"y":0.12,"z":-0.2,"w":0.2,"t":0.04,"spokes":0,"segments":8,"repeat":2,"dz":0.4,"far":0}'),
  ('turret','box','mat.white_composite',0,'hull','{"yaw":90,"x":0.95,"y":0.44,"w":0.4,"d":0.46,"h":0.18,"taper":0.4,"traverse":30,"far":0}'),
  ('rails','box','mat.graphite',0,'turret','{"y":0.09,"z":0.12,"w":0.12,"d":0.08,"h":1,"pitch":88,"recoil":0.06,"far":0}'),
  ('rail_glow','box','mat.plasma',0,'rails','{"y":0.2,"z":-0.045,"w":0.04,"d":0.02,"h":0.7,"far":0}'),
  ('ensign','box','mat.white_composite',1,'hull','{"x":-1.62,"y":0.8,"w":0.26,"d":0.015,"h":0.17,"far":0}'),
  ('ensign_staff','cylinder','mat.graphite',0,'hull','{"x":-1.5,"y":0.44,"w":0.03,"h":0.52,"segments":4,"far":0}'),
  ('spark','cone','mat.plasma',0,'rails','{"y":1.02,"w":0.16,"h":0.16,"segments":3,"flash":1,"far":0}'),
  ('streak','cylinder','mat.light_strip',0,'spark','{"y":0.16,"w":0.05,"h":1,"segments":4,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.36,"w":0.4,"h":0.36,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.hover_frigate_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.hover_frigate'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.hover_frigate_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
