-- bld.airfield@automation: Future airfield (mesh.airfield_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.airfield_automation','Future airfield');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.airfield_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('field','plinth','mat.turf',0,'','{"w":5.6,"d":5.6,"h":0.03,"steps":1,"tint":"#a8b890"}'),
  ('runway','box','mat.tarmac',0,'','{"z":1.4,"w":5.5,"d":1.3,"h":0.04,"tint":"#9aa0a4"}'),
  ('runway_marks','box','mat.canvas',0,'','{"x":-1.8,"y":0.04,"z":1.4,"w":0.4,"d":0.05,"h":0.01,"repeat":5,"dx":0.9,"far":0}'),
  ('threshold','box','mat.white_composite',1,'','{"x":-2.68,"y":0.04,"z":1.4,"w":0.12,"d":1.2,"h":0.01,"repeat":2,"dx":5.36}'),
  ('edge_lights','box','mat.light_strip',0,'','{"x":-2.5,"y":0.04,"z":0.78,"w":0.05,"d":0.05,"h":0.03,"repeat":9,"dx":0.625,"far":0}'),
  ('edge_lights_s','box','mat.light_strip',0,'','{"x":-2.5,"y":0.04,"z":2.02,"w":0.05,"d":0.05,"h":0.03,"repeat":9,"dx":0.625,"far":0}'),
  ('taxiway','box','mat.tarmac',0,'','{"x":-0.9,"z":0.45,"w":0.6,"d":0.7,"h":0.035,"tint":"#9aa0a4"}'),
  ('apron','box','mat.paving',0,'','{"x":-0.9,"z":-0.45,"w":3.4,"d":1.2,"h":0.03}'),
  ('hangar','box','mat.composite_panel',0,'','{"x":-1,"y":0.03,"z":-1.85,"w":2.8,"d":1.5,"h":0.75}'),
  ('hangar_roof','cylinder','mat.white_composite',0,'','{"x":-1,"y":0.78,"z":-2.6,"w":2.8,"h":1.5,"segments":16,"pitch":90,"bottom":1}'),
  ('hangar_doors','box','mat.white_composite',1,'','{"x":-1,"y":0.03,"z":-1.08,"w":2.2,"d":0.04,"h":1.25}'),
  ('hangar_glow','box','mat.light_strip',0,'','{"x":-1,"y":1.28,"z":-1.08,"w":2.2,"d":0.04,"h":0.04,"far":0}'),
  ('tower_shaft','cylinder','mat.composite_panel',0,'','{"x":1.6,"y":0.03,"z":-1.6,"w":0.55,"h":2.6,"taper":0.2,"segments":10}'),
  ('tower_band','cylinder','mat.white_composite',1,'','{"x":1.6,"y":2,"z":-1.6,"w":0.5,"h":0.3,"segments":10}'),
  ('cab_floor','cylinder','mat.white_composite',0,'','{"x":1.6,"y":2.63,"z":-1.6,"w":1.15,"h":0.1,"segments":12}'),
  ('cab','cylinder','mat.smart_glass',0,'','{"x":1.6,"y":2.73,"z":-1.6,"w":1.05,"h":0.42,"segments":12}'),
  ('cab_roof','dome','mat.white_composite',1,'','{"x":1.6,"y":3.15,"z":-1.6,"w":1.15,"h":0.3,"segments":12}'),
  ('cab_glow','cylinder','mat.light_strip',0,'','{"x":1.6,"y":2.7,"z":-1.6,"w":1.17,"h":0.03,"segments":12,"far":0}'),
  ('cab_aerial','cylinder','mat.white_composite',0,'','{"x":1.6,"y":3.45,"z":-1.6,"w":0.04,"h":0.6,"segments":4,"far":0}'),
  ('dish_post','cylinder','mat.white_composite',0,'','{"x":2.3,"y":0.03,"z":-0.35,"w":0.1,"h":1.2,"segments":6}'),
  ('dish','dome','mat.white_composite',0,'','{"x":2.3,"y":1.23,"z":-0.35,"yaw":20,"w":0.9,"h":0.1,"segments":10,"pitch":45}'),
  ('dish_back','dome','mat.white_composite',0,'','{"x":2.3,"y":1.23,"z":-0.35,"yaw":20,"w":0.9,"h":0.24,"segments":10,"pitch":-135,"far":0}'),
  ('pad_ring','cylinder','mat.light_strip',0,'','{"x":0.95,"y":0.03,"z":-0.45,"w":1.05,"h":0.03,"segments":12,"far":0}'),
  ('pad','cylinder','mat.graphite',0,'','{"x":0.95,"y":0.03,"z":-0.45,"w":0.95,"h":0.04,"segments":12}'),
  ('fuel','cylinder','mat.white_composite',0,'','{"x":0.55,"y":0.25,"z":-2.35,"w":0.42,"h":0.75,"segments":8,"roll":90}'),
  ('fuel_2','cylinder','mat.white_composite',0,'','{"x":0.55,"y":0.25,"z":-1.8,"w":0.42,"h":0.75,"segments":8,"roll":90,"far":0}'),
  ('jet','cylinder','mat.white_composite',0,'','{"x":-1.2,"y":0.24,"z":-0.95,"w":0.2,"h":1.1,"taper":0.3,"segments":6,"pitch":90}'),
  ('jet_nose','cone','mat.white_composite',0,'','{"x":-1.2,"y":0.24,"z":0.15,"w":0.14,"h":0.32,"segments":6,"pitch":90}'),
  ('jet_wings','box','mat.white_composite',0,'','{"x":-1.2,"y":0.22,"z":-0.75,"w":1,"d":0.6,"h":0.025,"taper":0.5}'),
  ('jet_fin','box','mat.white_composite',1,'','{"x":-1.2,"y":0.3,"z":-0.85,"w":0.025,"d":0.26,"h":0.3}'),
  ('jet_canopy','dome','mat.smart_glass',0,'','{"x":-1.2,"y":0.32,"z":-0.15,"w":0.14,"h":0.08,"segments":6,"far":0}'),
  ('jet_glow','cylinder','mat.light_strip',0,'','{"x":-1.2,"y":0.24,"z":-0.97,"w":0.18,"h":0.03,"segments":6,"pitch":90,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.airfield_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.airfield'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.airfield_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
