-- unit.fusion_tank@fusion: Fusion tank (mesh.fusion_tank_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fusion_tank_fusion','Fusion tank');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fusion_tank_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.white_composite',0,'','{"y":0.34,"z":-0.05,"w":1.5,"d":2.5,"h":0.4,"taper":0.12,"bank":4}'),
  ('belly','box','mat.graphite',0,'hull','{"y":-0.12,"w":1.3,"d":2.2,"h":0.12}'),
  ('glacis','box','mat.white_composite',0,'hull','{"z":1.22,"w":1.36,"d":0.05,"h":0.42,"pitch":-62}'),
  ('pads_front','wheel','mat.plasma',0,'hull','{"x":-0.72,"y":-0.14,"z":0.65,"roll":90,"w":0.44,"t":0.05,"spokes":0,"segments":8,"repeat":2,"dx":1}'),
  ('pads_back','wheel','mat.plasma',0,'hull','{"x":-0.72,"y":-0.14,"z":-0.75,"roll":90,"w":0.44,"t":0.05,"spokes":0,"segments":8,"repeat":2,"dx":1,"far":0}'),
  ('vents','box','mat.plasma',0,'hull','{"x":-0.74,"y":0.1,"z":-0.3,"w":0.02,"d":1.3,"h":0.05,"repeat":2,"dx":1.48,"far":0}'),
  ('skirts','box','mat.white_composite',1,'hull','{"x":-0.77,"y":0.18,"z":-0.05,"w":0.04,"d":2.3,"h":0.2,"repeat":2,"dx":1.54}'),
  ('turret','box','mat.white_composite',0,'hull','{"y":0.4,"z":-0.25,"w":1.15,"d":1.2,"h":0.34,"taper":0.2,"traverse":20}'),
  ('turret_band','box','mat.white_composite',1,'turret','{"y":0.1,"w":1.11,"d":1.15,"h":0.16}'),
  ('core','box','mat.plasma',0,'turret','{"y":0.1,"z":-0.6,"w":0.5,"d":0.02,"h":0.1}'),
  ('sensor','box','mat.graphite',0,'turret','{"x":0.3,"y":0.34,"z":-0.1,"w":0.18,"d":0.22,"h":0.12}'),
  ('sensor_eye','box','mat.light_strip',0,'sensor','{"y":0.03,"z":0.11,"w":0.12,"d":0.02,"h":0.05,"far":0}'),
  ('barrel_w','cylinder','mat.graphite',0,'turret','{"x":-0.15,"y":0.14,"z":0.5,"w":0.11,"h":1.6,"segments":6,"pitch":90,"recoil":0.1}'),
  ('barrel_e','cylinder','mat.graphite',0,'turret','{"x":0.15,"y":0.14,"z":0.5,"w":0.11,"h":1.6,"segments":6,"pitch":90,"recoil":0.1}'),
  ('coils_w','cylinder','mat.plasma',0,'barrel_w','{"y":0.35,"w":0.18,"h":0.08,"segments":6,"far":0}'),
  ('coils_e','cylinder','mat.plasma',0,'barrel_e','{"y":0.35,"w":0.18,"h":0.08,"segments":6,"far":0}'),
  ('tip_w','cylinder','mat.plasma',0,'barrel_w','{"y":1.5,"w":0.15,"h":0.1,"segments":6}'),
  ('tip_e','cylinder','mat.plasma',0,'barrel_e','{"y":1.5,"w":0.15,"h":0.1,"segments":6}'),
  ('spark_w','cone','mat.plasma',0,'barrel_w','{"y":1.62,"w":0.16,"h":0.16,"segments":3,"flash":1,"far":0}'),
  ('bolt_w','cylinder','mat.plasma',0,'spark_w','{"y":0.5,"w":0.12,"h":0.6,"segments":4,"far":0}'),
  ('spark_e','cone','mat.plasma',0,'barrel_e','{"y":1.62,"w":0.16,"h":0.16,"segments":3,"flash":1,"far":0}'),
  ('bolt_e','cylinder','mat.plasma',0,'spark_e','{"y":0.5,"w":0.12,"h":0.6,"segments":4,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fusion_tank_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.fusion_tank'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.fusion_tank_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
