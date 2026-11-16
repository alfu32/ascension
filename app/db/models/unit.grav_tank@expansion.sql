-- unit.grav_tank@expansion: Grav Tank (mesh.grav_tank_expansion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.grav_tank_expansion','Grav Tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.grav_tank_expansion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.white_composite',0,'','{"y":0.62,"z":0,"w":1.75,"d":2.7,"h":0.36,"taper":0.22,"bank":3}'),
  ('belly','box','mat.graphite',0,'hull','{"y":-0.16,"w":1.45,"d":2.3,"h":0.16,"taper":-0.1}'),
  ('prow','box','mat.white_composite',0,'hull','{"z":1.48,"w":1.3,"d":0.5,"h":0.3,"taper":0.5}'),
  ('sponsons','box','mat.white_composite',1,'hull','{"x":-0.98,"y":0.02,"z":-0.2,"w":0.3,"d":1.9,"h":0.24,"taper":0.3,"repeat":2,"dx":1.96}'),
  ('lift_rings','wheel','mat.light_strip',0,'hull','{"x":-0.45,"y":-0.2,"z":0.75,"roll":90,"w":0.6,"t":0.05,"spokes":0,"segments":10,"repeat":2,"dx":0.9}'),
  ('lift_rings_aft','wheel','mat.light_strip',0,'hull','{"x":-0.45,"y":-0.2,"z":-0.75,"roll":90,"w":0.6,"t":0.05,"spokes":0,"segments":10,"repeat":2,"dx":0.9,"far":0}'),
  ('glow_strips','box','mat.light_strip',0,'hull','{"x":-1.13,"y":0.1,"z":-0.2,"w":0.02,"d":1.6,"h":0.04,"repeat":2,"dx":2.26,"far":0}'),
  ('turret','dome','mat.white_composite',0,'hull','{"y":0.32,"z":-0.25,"w":1.25,"h":0.5,"segments":10,"traverse":20}'),
  ('turret_band','cylinder','mat.white_composite',1,'turret','{"y":0,"w":1.27,"h":0.14,"segments":10}'),
  ('sensor','box','mat.graphite',0,'turret','{"x":-0.32,"y":0.36,"z":-0.05,"w":0.2,"d":0.24,"h":0.13}'),
  ('sensor_eye','box','mat.light_strip',0,'sensor','{"y":0.04,"z":0.12,"w":0.14,"d":0.02,"h":0.05,"far":0}'),
  ('lance','cylinder','mat.graphite',0,'turret','{"y":0.24,"z":0.35,"w":0.16,"h":1.75,"segments":6,"pitch":90,"recoil":0.12}'),
  ('lance_coils','cylinder','mat.light_strip',0,'lance','{"y":0.3,"w":0.24,"h":0.07,"segments":6,"repeat":1,"far":0}'),
  ('lance_coil_2','cylinder','mat.light_strip',0,'lance','{"y":0.75,"w":0.22,"h":0.07,"segments":6,"far":0}'),
  ('lance_tip','cylinder','mat.white_composite',0,'lance','{"y":1.65,"w":0.2,"h":0.14,"segments":6}'),
  ('spark','cone','mat.light_strip',0,'lance','{"y":1.8,"w":0.22,"h":0.16,"segments":4,"flash":1,"far":0}'),
  ('beam','cylinder','mat.light_strip',0,'spark','{"y":0.12,"w":0.07,"h":1.2,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.grav_tank_expansion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.grav_tank'), NULL, (SELECT id FROM def WHERE key='age.expansion'), (SELECT id FROM def WHERE key='mesh.grav_tank_expansion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
