-- unit.interceptor@fusion: Interceptor (mesh.interceptor_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.interceptor_fusion','Interceptor');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.interceptor_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('fuselage','box','mat.white_composite',0,'','{"y":4.6,"z":-0.1,"w":0.28,"d":2.6,"h":0.26,"taper":0.3,"bank":28}'),
  ('nose','cone','mat.white_composite',0,'fuselage','{"y":0.12,"z":1.3,"w":0.22,"h":0.9,"segments":7,"pitch":90}'),
  ('canopy','box','mat.glass',0,'fuselage','{"y":0.24,"z":0.7,"w":0.16,"d":0.55,"h":0.12,"taper":0.5,"tint":"#4a6a88"}'),
  ('wing_e','box','mat.white_composite',0,'fuselage','{"y":0.04,"z":-0.35,"w":0.04,"d":1.4,"h":1.1,"taper":0.85,"roll":90,"yaw":55}'),
  ('wing_w','box','mat.white_composite',0,'fuselage','{"y":0.04,"z":-0.35,"w":0.04,"d":1.4,"h":1.1,"taper":0.85,"roll":-90,"yaw":-55}'),
  ('tip_e','box','mat.white_composite',1,'wing_e','{"y":0.62,"w":0.045,"d":0.62,"h":0.36,"taper":0.5}'),
  ('tip_w','box','mat.white_composite',1,'wing_w','{"y":0.62,"w":0.045,"d":0.62,"h":0.36,"taper":0.5}'),
  ('canard_e','box','mat.white_composite',0,'fuselage','{"y":0.1,"z":0.6,"w":0.03,"d":0.25,"h":0.32,"taper":0.5,"roll":90,"yaw":30}'),
  ('canard_w','box','mat.white_composite',0,'fuselage','{"y":0.1,"z":0.6,"w":0.03,"d":0.25,"h":0.32,"taper":0.5,"roll":-90,"yaw":-30}'),
  ('fin_e','box','mat.white_composite',1,'fuselage','{"x":0.12,"y":0.22,"z":-1,"w":0.03,"d":0.55,"h":0.5,"taper":0.6,"roll":20,"pitch":-25}'),
  ('fin_w','box','mat.white_composite',1,'fuselage','{"x":-0.12,"y":0.22,"z":-1,"w":0.03,"d":0.55,"h":0.5,"taper":0.6,"roll":-20,"pitch":-25}'),
  ('spine','box','mat.light_strip',0,'fuselage','{"y":0.26,"z":-0.3,"w":0.03,"d":0.9,"h":0.02,"far":0}'),
  ('nacelles','cylinder','mat.graphite',0,'fuselage','{"x":-0.12,"y":0.12,"z":-1.28,"w":0.16,"h":0.18,"segments":6,"pitch":-90,"repeat":2,"dx":0.24,"far":0}'),
  ('drives','wheel','mat.plasma',0,'fuselage','{"x":-0.12,"y":0.05,"z":-1.47,"yaw":90,"w":0.14,"t":0.03,"spokes":3,"segments":6,"repeat":2,"dx":0.24}'),
  ('gun','cylinder','mat.graphite',0,'fuselage','{"x":0.17,"y":0.06,"z":0.5,"w":0.03,"h":0.3,"segments":4,"pitch":90,"far":0}'),
  ('spark','cone','mat.plasma',0,'gun','{"y":0.3,"w":0.1,"h":0.1,"segments":3,"flash":2,"far":0}'),
  ('bolt','cylinder','mat.plasma',0,'spark','{"y":0.6,"w":0.07,"h":0.55,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.interceptor_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.interceptor'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.interceptor_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
