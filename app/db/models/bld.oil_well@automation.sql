-- bld.oil_well@automation: Future oil well (mesh.oil_well_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.oil_well_automation','Future oil well');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.oil_well_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":1.8,"d":1.8,"h":0.04,"steps":1}'),
  ('pool','cylinder','mat.crude',0,'','{"x":-0.45,"y":0.04,"z":0.5,"w":0.45,"h":0.01,"segments":8,"far":0}'),
  ('skid','box','mat.white_composite',0,'','{"x":-0.05,"y":0.04,"z":-0.2,"w":1.35,"d":0.4,"h":0.08}'),
  ('skid_glow','box','mat.light_strip',0,'','{"x":-0.05,"y":0.06,"z":-0.2,"w":1.37,"d":0.42,"h":0.02,"far":0}'),
  ('samson','box','mat.white_composite',0,'','{"x":0,"y":0.12,"z":-0.38,"w":0.09,"d":0.09,"h":0.9,"pitch":12}'),
  ('samson_s','box','mat.white_composite',0,'','{"x":0,"y":0.12,"z":-0.02,"w":0.09,"d":0.09,"h":0.9,"pitch":-12}'),
  ('beam','box','mat.white_composite',1,'','{"x":-0.15,"y":0.98,"z":-0.2,"w":1.25,"d":0.13,"h":0.14,"roll":5}'),
  ('head','box','mat.white_composite',0,'','{"x":-0.78,"y":0.7,"z":-0.2,"w":0.15,"d":0.15,"h":0.42,"roll":-12}'),
  ('rod','cylinder','mat.steel',0,'','{"x":-0.83,"y":0.12,"z":-0.2,"w":0.03,"h":0.6,"segments":4}'),
  ('wellhead','cylinder','mat.graphite',0,'','{"x":-0.83,"y":0.04,"z":-0.2,"w":0.16,"h":0.16,"segments":6}'),
  ('gearbox','box','mat.white_composite',0,'','{"x":0.45,"y":0.12,"z":-0.2,"w":0.28,"d":0.24,"h":0.28}'),
  ('cranks','shield','mat.graphite',0,'','{"x":0.45,"y":0.42,"z":-0.4,"w":0.5,"t":0.05,"boss":0.04,"segments":8,"rim":"#c8ced2","repeat":2,"dz":0.4}'),
  ('pitmans','box','mat.graphite',0,'','{"x":0.42,"y":0.45,"z":-0.42,"w":0.04,"d":0.04,"h":0.55,"repeat":2,"dz":0.44,"far":0}'),
  ('motor','box','mat.graphite',0,'','{"x":0.73,"y":0.12,"z":-0.2,"w":0.2,"d":0.2,"h":0.16,"far":0}'),
  ('tank','cylinder','mat.white_composite',0,'','{"x":0.52,"y":0.04,"z":0.52,"w":0.44,"h":0.42,"segments":10}'),
  ('tank_top','dome','mat.white_composite',0,'','{"x":0.52,"y":0.46,"z":0.52,"w":0.44,"h":0.16,"segments":10}'),
  ('tank_band','cylinder','mat.white_composite',1,'','{"x":0.52,"y":0.24,"z":0.52,"w":0.46,"h":0.12,"segments":10}'),
  ('tank_2','cylinder','mat.white_composite',0,'','{"x":0.04,"y":0.04,"z":0.58,"w":0.36,"h":0.36,"segments":10}'),
  ('tank_2_top','dome','mat.white_composite',0,'','{"x":0.04,"y":0.4,"z":0.58,"w":0.36,"h":0.13,"segments":10,"far":0}'),
  ('solar','box','mat.solar_panel',0,'','{"x":-0.55,"y":0.2,"z":0.45,"w":0.5,"d":0.4,"h":0.03,"pitch":25,"far":0}'),
  ('pipe_run','box','mat.graphite',0,'','{"x":0.1,"y":0.05,"z":0.18,"w":1,"d":0.05,"h":0.05,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.oil_well_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.oil_well'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.oil_well_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
