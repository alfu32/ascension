-- bld.nuclear_reactor@automation: Nuclear reactor (mesh.nuclear_reactor_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.nuclear_reactor_automation','Nuclear reactor');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.nuclear_reactor_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":3.7,"d":3.7,"h":0.05,"steps":1}'),
  ('containment','cylinder','mat.white_composite',0,'','{"x":-0.85,"y":0.05,"z":-0.7,"w":1.7,"h":1.1,"segments":16}'),
  ('containment_dome','dome','mat.white_composite',1,'','{"x":-0.85,"y":1.15,"z":-0.7,"w":1.7,"h":0.8,"segments":16}'),
  ('containment_band','cylinder','mat.white_composite',1,'','{"x":-0.85,"y":0.9,"z":-0.7,"w":1.72,"h":0.16,"segments":16}'),
  ('containment_glow','cylinder','mat.light_strip',0,'','{"x":-0.85,"y":0.25,"z":-0.7,"w":1.72,"h":0.03,"segments":16,"far":0}'),
  ('containment_glass','cylinder','mat.smart_glass',0,'','{"x":-0.85,"y":0.55,"z":-0.7,"w":1.71,"h":0.14,"segments":16,"far":0}'),
  ('crane_rail','torus','mat.graphite',0,'','{"x":-0.85,"y":1.5,"z":-0.7,"w":1.4,"t":0.06,"segments":16,"sides":4,"far":0}'),
  ('beacon','cylinder','mat.light_strip',0,'','{"x":-0.85,"y":1.93,"z":-0.7,"w":0.22,"h":0.1,"segments":8,"far":0}'),
  ('airlock','box','mat.composite_panel',0,'','{"x":-0.85,"y":0.05,"z":0.25,"w":0.6,"d":0.4,"h":0.6}'),
  ('airlock_door','box','mat.smart_glass',0,'','{"x":-0.85,"y":0.05,"z":0.46,"w":0.34,"d":0.02,"h":0.42,"far":0}'),
  ('cooling','cylinder','mat.white_composite',0,'','{"x":0.95,"y":0.05,"z":-0.85,"w":1.5,"h":1.9,"taper":0.3,"segments":16}'),
  ('cooling_top','cylinder','mat.white_composite',1,'','{"x":0.95,"y":2.55,"z":-0.85,"w":1.25,"h":0.6,"taper":0.16,"segments":16,"pitch":180}'),
  ('cooling_glow','cylinder','mat.light_strip',0,'','{"x":0.95,"y":0.3,"z":-0.85,"w":1.47,"h":0.03,"segments":16,"far":0}'),
  ('cooling_mouth','cylinder','mat.graphite',0,'','{"x":0.95,"y":2.55,"z":-0.85,"w":1.15,"h":0.01,"segments":16,"far":0}'),
  ('cooling_rim','torus','mat.light_strip',0,'','{"x":0.95,"y":2.55,"z":-0.85,"w":1.2,"t":0.04,"segments":16,"sides":4,"far":0}'),
  ('cooling_legs','box','mat.graphite',0,'','{"x":0.2,"y":0.05,"z":-0.85,"w":0.08,"d":0.08,"h":0.22,"repeat":2,"dx":1.5,"far":0}'),
  ('steam_line','cylinder','mat.white_composite',0,'','{"x":0.95,"y":0.4,"z":-0.15,"w":0.22,"h":0.85,"segments":8,"pitch":90,"far":0}'),
  ('duct','cylinder','mat.white_composite',0,'','{"x":-0.85,"y":0.75,"z":0.0,"w":0.3,"h":0.62,"segments":8,"pitch":90}'),
  ('hall','box','mat.composite_panel',0,'','{"x":-0.05,"y":0.05,"z":1.15,"w":3.3,"d":1.0,"h":0.75}'),
  ('hall_glass','box','mat.smart_glass',0,'','{"x":-0.05,"y":0.3,"z":1.15,"w":3.32,"d":1.02,"h":0.2}'),
  ('hall_band','box','mat.white_composite',1,'','{"x":-0.05,"y":0.7,"z":1.15,"w":3.34,"d":1.04,"h":0.1}'),
  ('hall_glow','box','mat.light_strip',0,'','{"x":-0.05,"y":0.1,"z":1.15,"w":3.34,"d":1.04,"h":0.03,"far":0}'),
  ('hall_vault','cylinder','mat.white_composite',0,'','{"x":-1.7,"y":0.8,"z":1.15,"w":0.96,"h":3.3,"segments":12,"roll":90}'),
  ('hall_skylight','box','mat.solar_panel',0,'','{"x":-0.05,"y":1.26,"z":1.15,"w":3.0,"d":0.3,"h":0.03,"far":0}'),
  ('transformers','box','mat.graphite',0,'','{"x":1.3,"y":0.05,"z":0.3,"w":0.28,"d":0.28,"h":0.4,"repeat":2,"dx":0.35}'),
  ('transformer_fins','box','mat.white_composite',0,'','{"x":1.3,"y":0.1,"z":0.3,"w":0.32,"d":0.32,"h":0.04,"repeat":2,"dx":0.35,"far":0}'),
  ('pylon','cylinder','mat.white_composite',0,'','{"x":1.6,"y":0.05,"z":-0.05,"w":0.08,"h":1.5,"segments":6}'),
  ('pylon_arm','box','mat.white_composite',0,'','{"x":1.6,"y":1.45,"z":-0.05,"w":0.08,"d":0.6,"h":0.06,"far":0}'),
  ('pylon_light','cylinder','mat.light_strip',0,'','{"x":1.6,"y":1.55,"z":-0.05,"w":0.1,"h":0.05,"segments":6,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":-1.65,"y":0.05,"z":0.35,"h":2.2,"fw":0.5,"fh":0.7,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.nuclear_reactor_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.nuclear_reactor'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.nuclear_reactor_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
