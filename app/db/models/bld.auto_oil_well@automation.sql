-- bld.auto_oil_well@automation: Automated Oil Well (mesh.auto_oil_well_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.auto_oil_well_automation','Automated Oil Well');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.auto_oil_well_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":1.8,"d":1.8,"h":0.04,"steps":1}'),
  ('pad','cylinder','mat.white_composite',0,'','{"x":-0.35,"y":0.04,"z":-0.35,"w":0.9,"h":0.12,"segments":10}'),
  ('pod','dome','mat.white_composite',0,'','{"x":-0.35,"y":0.16,"z":-0.35,"w":0.62,"h":0.4,"segments":10}'),
  ('pod_glow','cylinder','mat.light_strip',0,'','{"x":-0.35,"y":0.16,"z":-0.35,"w":0.64,"h":0.03,"segments":10,"far":0}'),
  ('leg_1','box','mat.white_composite',0,'','{"x":-0.65,"y":0.16,"z":-0.65,"w":0.07,"d":0.07,"h":2.3,"roll":5,"pitch":5}'),
  ('leg_2','box','mat.white_composite',0,'','{"x":-0.05,"y":0.16,"z":-0.65,"w":0.07,"d":0.07,"h":2.3,"roll":-5,"pitch":5}'),
  ('leg_3','box','mat.white_composite',0,'','{"x":-0.65,"y":0.16,"z":-0.05,"w":0.07,"d":0.07,"h":2.3,"roll":5,"pitch":-5}'),
  ('leg_4','box','mat.white_composite',0,'','{"x":-0.05,"y":0.16,"z":-0.05,"w":0.07,"d":0.07,"h":2.3,"roll":-5,"pitch":-5}'),
  ('band','box','mat.white_composite',1,'','{"x":-0.35,"y":1,"z":-0.35,"w":0.5,"d":0.5,"h":0.1}'),
  ('crown','box','mat.white_composite',1,'','{"x":-0.35,"y":2.4,"z":-0.35,"w":0.3,"d":0.3,"h":0.2}'),
  ('crown_light','box','mat.light_strip',0,'','{"x":-0.35,"y":2.6,"z":-0.35,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('tanks','cylinder','mat.white_composite',0,'','{"x":0.05,"y":0.24,"z":0.25,"w":0.4,"h":0.8,"segments":10,"roll":90,"repeat":2,"dz":0.45}'),
  ('tank_bands','cylinder','mat.white_composite',1,'','{"x":0.35,"y":0.24,"z":0.25,"w":0.42,"h":0.14,"segments":10,"roll":90,"repeat":2,"dz":0.45}'),
  ('tank_glow','box','mat.light_strip',0,'','{"x":0.45,"y":0.04,"z":0.47,"w":0.8,"d":0.04,"h":0.02,"far":0}'),
  ('pipe','box','mat.graphite',0,'','{"x":0.1,"y":0.08,"z":-0.1,"w":0.5,"d":0.06,"h":0.06,"far":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":0.45,"y":0.3,"z":-0.5,"w":0.65,"d":0.55,"h":0.03,"pitch":22}'),
  ('solar_post','box','mat.graphite',0,'','{"x":0.45,"y":0.04,"z":-0.5,"w":0.05,"d":0.05,"h":0.27,"far":0}'),
  ('valves','box','mat.graphite',0,'','{"x":-0.55,"y":0.04,"z":0.5,"w":0.3,"d":0.3,"h":0.22}'),
  ('valve_light','box','mat.light_strip',0,'','{"x":-0.55,"y":0.26,"z":0.5,"w":0.1,"d":0.1,"h":0.03,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.auto_oil_well_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.auto_oil_well'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.auto_oil_well_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
