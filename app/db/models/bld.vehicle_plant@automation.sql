-- bld.vehicle_plant@automation: Future vehicle plant (mesh.vehicle_plant_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.vehicle_plant_automation','Future vehicle plant');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.vehicle_plant_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('hall','box','mat.composite_panel',0,'','{"y":0.05,"z":-0.7,"w":3.4,"d":2,"h":1.35}'),
  ('hall_vault','cylinder','mat.white_composite',0,'','{"x":-1.7,"y":1.4,"z":-0.7,"w":2,"h":3.4,"segments":16,"roll":90}'),
  ('vault_stripe','box','mat.white_composite',1,'','{"y":2.36,"z":-0.7,"w":3.44,"d":0.5,"h":0.06}'),
  ('doors','box','mat.white_composite',0,'','{"x":-0.8,"y":0.05,"z":0.31,"w":1.2,"d":0.04,"h":1.15,"repeat":2,"dx":1.6,"tint":"#c8d0d4"}'),
  ('door_frames','box','mat.white_composite',1,'','{"x":-0.8,"y":1.2,"z":0.33,"w":1.3,"d":0.04,"h":0.16,"repeat":2,"dx":1.6}'),
  ('door_glow','box','mat.light_strip',0,'','{"x":-0.8,"y":1.18,"z":0.36,"w":1.2,"d":0.02,"h":0.02,"repeat":2,"dx":1.6,"far":0}'),
  ('vent_stack','cylinder','mat.white_composite',0,'','{"x":1.2,"y":0.05,"z":-1.4,"w":0.22,"h":3,"segments":6}'),
  ('vent_light','cylinder','mat.light_strip',0,'','{"x":1.2,"y":3.05,"z":-1.4,"w":0.24,"h":0.04,"segments":6,"far":0}'),
  ('crane_posts','box','mat.white_composite',0,'','{"x":-1.55,"y":0.05,"z":1.65,"w":0.12,"d":0.12,"h":1.6,"repeat":2,"dx":3.1}'),
  ('crane_rails','box','mat.white_composite',0,'','{"x":-1.55,"y":1.6,"z":1,"w":0.12,"d":1.5,"h":0.12,"repeat":2,"dx":3.1}'),
  ('crane_bridge','box','mat.white_composite',1,'','{"y":1.72,"z":1.05,"w":3.24,"d":0.2,"h":0.18}'),
  ('crane_trolley','box','mat.graphite',0,'','{"x":-0.8,"y":1.55,"z":1.05,"w":0.3,"d":0.3,"h":0.17,"far":0}'),
  ('crane_hook','cylinder','mat.steel',0,'','{"x":-0.8,"y":1,"z":1.05,"w":0.03,"h":0.55,"segments":3,"far":0}'),
  ('module','box','mat.white_composite',0,'','{"x":-0.8,"y":0.75,"z":1.05,"w":0.34,"d":0.4,"h":0.26,"tint":"#c8d0d4"}'),
  ('rover','box','mat.white_composite',0,'','{"x":-0.8,"y":0.2,"z":1.05,"w":0.5,"d":1,"h":0.16}'),
  ('rover_cab','dome','mat.smart_glass',0,'','{"x":-0.8,"y":0.36,"z":1.3,"w":0.4,"h":0.22,"segments":8}'),
  ('rover_wheels','cylinder','mat.rubber',0,'','{"x":-1.1,"y":0.17,"z":0.7,"w":0.34,"h":0.08,"segments":8,"roll":90,"repeat":2,"dx":0.52}'),
  ('rover_wheels_b','cylinder','mat.rubber',0,'','{"x":-1.1,"y":0.17,"z":1.4,"w":0.34,"h":0.08,"segments":8,"roll":90,"repeat":2,"dx":0.52,"far":0}'),
  ('tank_hull','box','mat.white_composite',0,'','{"x":0.75,"y":0.2,"z":1.05,"w":0.62,"d":1.05,"h":0.2,"taper":0.12}'),
  ('tank_glow','box','mat.light_strip',0,'','{"x":0.75,"y":0.08,"z":1.05,"w":0.56,"d":0.95,"h":0.12,"far":0}'),
  ('tank_turret','box','mat.white_composite',0,'','{"x":0.75,"y":0.4,"z":0.98,"w":0.4,"d":0.48,"h":0.16,"taper":0.25}'),
  ('tank_gun','box','mat.graphite',0,'','{"x":0.75,"y":0.46,"z":1.15,"w":0.07,"d":0.07,"h":0.6,"pitch":90}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.vehicle_plant_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.vehicle_plant'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.vehicle_plant_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
