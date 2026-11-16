-- unit.laser_battery@fusion: Laser battery (mesh.laser_battery_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.laser_battery_fusion','Laser battery');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.laser_battery_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.56,"z":0.78,"w":0.5,"t":0.18,"spokes":0,"segments":8,"repeat":2,"dx":1.12}'),
  ('wheels_mid','wheel','mat.rubber',0,'','{"x":-0.56,"z":0.02,"w":0.5,"t":0.18,"spokes":0,"segments":8,"repeat":2,"dx":1.12,"far":0}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.56,"z":-0.74,"w":0.5,"t":0.18,"spokes":0,"segments":8,"repeat":2,"dx":1.12}'),
  ('chassis','box','mat.graphite',0,'','{"y":0.2,"w":0.86,"d":2.2,"h":0.14}'),
  ('body','box','mat.white_composite',0,'','{"y":0.34,"z":-0.25,"w":1.24,"d":1.6,"h":0.36}'),
  ('fenders','box','mat.white_composite',0,'','{"x":-0.56,"y":0.5,"w":0.24,"d":2.2,"h":0.04,"repeat":2,"dx":1.12,"far":0}'),
  ('cab','box','mat.white_composite',0,'','{"y":0.34,"z":0.82,"w":1.12,"d":0.56,"h":0.5,"taper":0.18}'),
  ('windscreen','box','mat.smart_glass',0,'','{"y":0.54,"z":1.09,"w":0.86,"d":0.03,"h":0.2,"pitch":-18,"far":0}'),
  ('cab_roof','box','mat.white_composite',1,'','{"y":0.84,"z":0.8,"w":0.84,"d":0.44,"h":0.03}'),
  ('base','cylinder','mat.white_composite',0,'','{"y":0.7,"z":-0.35,"w":0.92,"h":0.14,"segments":10,"traverse":40}'),
  ('turret','box','mat.white_composite',1,'base','{"y":0.14,"w":0.72,"d":0.72,"h":0.34,"taper":0.25}'),
  ('radiators','box','mat.graphite',0,'base','{"x":-0.44,"y":0.16,"w":0.05,"d":0.62,"h":0.36,"repeat":2,"dx":0.88,"far":0}'),
  ('emitter','cylinder','mat.graphite',0,'base','{"y":0.36,"z":-0.1,"w":0.3,"h":1.15,"segments":8,"pitch":38}'),
  ('collar','cylinder','mat.plasma',0,'emitter','{"y":0.32,"w":0.36,"h":0.06,"segments":8,"far":0}'),
  ('collar_2','cylinder','mat.plasma',0,'emitter','{"y":0.56,"w":0.36,"h":0.06,"segments":8,"far":0}'),
  ('hood','cylinder','mat.white_composite',0,'emitter','{"y":0.88,"w":0.42,"h":0.27,"taper":-0.15,"segments":8,"far":0}'),
  ('lens','cylinder','mat.laser',0,'emitter','{"y":1.12,"w":0.32,"h":0.04,"segments":8,"far":0}'),
  ('beam','cylinder','mat.laser',0,'emitter','{"y":1.16,"w":0.08,"h":2.4,"segments":4,"flash":1,"far":0}'),
  ('glare','cone','mat.laser',0,'beam','{"w":0.42,"h":0.3,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.laser_battery_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.laser_battery'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.laser_battery_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
