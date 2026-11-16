-- bld.defence_turret@automation: Defence Turret (mesh.defence_turret_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.defence_turret_automation','Defence Turret');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.defence_turret_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','cylinder','mat.composite_panel',0,'','{"w":1.8,"h":0.55,"segments":8,"taper":0.08}'),
  ('base_band','cylinder','mat.white_composite',1,'','{"y":0.36,"w":1.72,"h":0.13,"segments":8}'),
  ('base_glow','cylinder','mat.light_strip',0,'','{"y":0.55,"w":1.6,"h":0.03,"segments":8,"far":0}'),
  ('deck','cylinder','mat.graphite',0,'','{"y":0.55,"w":1.3,"h":0.08,"segments":12}'),
  ('neck','cylinder','mat.white_composite',0,'','{"y":0.63,"w":0.75,"h":0.22,"segments":10}'),
  ('turret','box','mat.white_composite',0,'','{"y":0.85,"z":-0.05,"w":0.95,"d":1.15,"h":0.46,"taper":0.3}'),
  ('turret_band','box','mat.white_composite',1,'','{"y":0.9,"z":-0.05,"w":0.98,"d":0.75,"h":0.2}'),
  ('mantlet','box','mat.graphite',0,'','{"y":0.9,"z":0.47,"w":0.6,"d":0.12,"h":0.3,"far":0}'),
  ('barrels','cylinder','mat.graphite',0,'','{"x":-0.17,"y":1.05,"z":0.4,"w":0.12,"h":1.15,"segments":6,"pitch":84,"repeat":2,"dx":0.34}'),
  ('shrouds','box','mat.white_composite',0,'','{"x":-0.17,"y":0.98,"z":0.75,"w":0.18,"d":0.5,"h":0.14,"repeat":2,"dx":0.34,"far":0}'),
  ('muzzles','cylinder','mat.plasma',0,'','{"x":-0.17,"y":1.17,"z":1.54,"w":0.13,"h":0.04,"segments":6,"pitch":84,"repeat":2,"dx":0.34,"far":0}'),
  ('cap','box','mat.white_composite',1,'','{"y":1.31,"z":-0.05,"w":0.62,"d":0.74,"h":0.03}'),
  ('sensor','dome','mat.white_composite',0,'','{"x":0.22,"y":1.34,"z":-0.3,"w":0.3,"h":0.18,"segments":8}'),
  ('sensor_eye','box','mat.light_strip',0,'','{"x":0.22,"y":1.38,"z":-0.16,"w":0.14,"d":0.02,"h":0.05,"far":0}'),
  ('vents','box','mat.graphite',0,'','{"y":0.95,"z":-0.63,"w":0.5,"d":0.04,"h":0.2,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.defence_turret_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.defence_turret'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.defence_turret_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
