-- bld.fusion_plant@fusion: Fusion Plant (mesh.fusion_plant_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fusion_plant_fusion','Fusion Plant');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fusion_plant_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.05,"steps":1}'),
  ('pedestal','cylinder','mat.composite_panel',0,'','{"x":-0.55,"y":0.05,"z":-0.4,"w":2.5,"h":0.3,"segments":16}'),
  ('pedestal_glow','cylinder','mat.light_strip',0,'','{"x":-0.55,"y":0.33,"z":-0.4,"w":2.52,"h":0.03,"segments":16,"far":0}'),
  ('torus','torus','mat.white_composite',0,'','{"x":-0.55,"y":0.35,"z":-0.4,"w":2.3,"t":0.7,"segments":24,"sides":8}'),
  ('plasma_ring','torus','mat.plasma',0,'','{"x":-0.55,"y":0.62,"z":-0.4,"w":2.34,"t":0.12,"segments":24,"sides":4,"far":0}'),
  ('plasma_core','torus','mat.plasma',0,'','{"x":-0.55,"y":0.45,"z":-0.4,"w":1.1,"t":0.3,"segments":16,"sides":6,"far":0}'),
  ('solenoid','cylinder','mat.graphite',0,'','{"x":-0.55,"y":0.35,"z":-0.4,"w":0.36,"h":1.1,"segments":8}'),
  ('solenoid_cap','cylinder','mat.plasma',0,'','{"x":-0.55,"y":1.45,"z":-0.4,"w":0.38,"h":0.05,"segments":8,"far":0}'),
  ('coil_1','box','mat.white_composite',1,'','{"x":0.25,"y":0.32,"z":-0.4,"yaw":90,"w":0.11,"d":0.78,"h":0.76}'),
  ('coil_2','box','mat.white_composite',1,'','{"x":0.0157,"y":0.32,"z":0.1657,"yaw":45,"w":0.11,"d":0.78,"h":0.76,"far":0}'),
  ('coil_3','box','mat.white_composite',1,'','{"x":-0.55,"y":0.32,"z":0.4,"yaw":0,"w":0.11,"d":0.78,"h":0.76}'),
  ('coil_4','box','mat.white_composite',1,'','{"x":-1.1157,"y":0.32,"z":0.1657,"yaw":-45,"w":0.11,"d":0.78,"h":0.76,"far":0}'),
  ('coil_5','box','mat.white_composite',1,'','{"x":-1.35,"y":0.32,"z":-0.4,"yaw":-90,"w":0.11,"d":0.78,"h":0.76}'),
  ('coil_6','box','mat.white_composite',1,'','{"x":-1.1157,"y":0.32,"z":-0.9657,"yaw":-135,"w":0.11,"d":0.78,"h":0.76,"far":0}'),
  ('coil_7','box','mat.white_composite',1,'','{"x":-0.55,"y":0.32,"z":-1.2,"yaw":-180,"w":0.11,"d":0.78,"h":0.76}'),
  ('coil_8','box','mat.white_composite',1,'','{"x":0.0157,"y":0.32,"z":-0.9657,"yaw":-225,"w":0.11,"d":0.78,"h":0.76,"far":0}'),
  ('cooling','cylinder','mat.white_composite',0,'','{"x":1.3,"y":0.05,"z":-1.1,"w":1,"h":1.45,"taper":0.32,"segments":12}'),
  ('cooling_top','cylinder','mat.white_composite',1,'','{"x":1.3,"y":1.95,"z":-1.1,"w":0.84,"h":0.45,"taper":0.19,"segments":12,"pitch":180}'),
  ('cooling_glow','cylinder','mat.light_strip',0,'','{"x":1.3,"y":0.25,"z":-1.1,"w":0.96,"h":0.03,"segments":12,"far":0}'),
  ('cooling_mouth','cylinder','mat.graphite',0,'','{"x":1.3,"y":1.95,"z":-1.1,"w":0.74,"h":0.01,"segments":12,"far":0}'),
  ('cooling_2','cylinder','mat.white_composite',0,'','{"x":1.3,"y":0.05,"z":0.6,"w":1,"h":1.45,"taper":0.32,"segments":12}'),
  ('cooling_2_top','cylinder','mat.white_composite',1,'','{"x":1.3,"y":1.95,"z":0.6,"w":0.84,"h":0.45,"taper":0.19,"segments":12,"pitch":180}'),
  ('cooling_2_glow','cylinder','mat.light_strip',0,'','{"x":1.3,"y":0.25,"z":0.6,"w":0.96,"h":0.03,"segments":12,"far":0}'),
  ('cooling_2_mouth','cylinder','mat.graphite',0,'','{"x":1.3,"y":1.95,"z":0.6,"w":0.74,"h":0.01,"segments":12,"far":0}'),
  ('hall','box','mat.composite_panel',0,'','{"x":-0.65,"y":0.05,"z":1.35,"w":2.3,"d":0.8,"h":0.7}'),
  ('hall_glass','box','mat.smart_glass',0,'','{"x":-0.65,"y":0.3,"z":1.35,"w":2.32,"d":0.82,"h":0.2}'),
  ('hall_band','box','mat.white_composite',1,'','{"x":-0.65,"y":0.75,"z":1.35,"w":2.34,"d":0.84,"h":0.12}'),
  ('hall_solar','box','mat.solar_panel',0,'','{"x":-0.65,"y":0.87,"z":1.35,"w":2.1,"d":0.6,"h":0.03,"pitch":12,"far":0}'),
  ('duct','cylinder','mat.white_composite',0,'','{"x":0.45,"y":0.45,"z":-0.6,"w":0.18,"h":0.45,"segments":6,"roll":90,"far":0}'),
  ('duct_2','cylinder','mat.white_composite',0,'','{"x":0.45,"y":0.45,"z":0.2,"w":0.18,"h":0.45,"segments":6,"roll":90,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fusion_plant_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fusion_plant'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.fusion_plant_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
