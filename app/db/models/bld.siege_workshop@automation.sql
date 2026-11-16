-- bld.siege_workshop@automation: Future siege workshop (mesh.siege_workshop_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.siege_workshop_automation','Future siege workshop');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.siege_workshop_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('shop','box','mat.composite_panel',0,'','{"y":0.05,"z":-0.85,"w":3.3,"d":1.8,"h":1.4,"door":1.2,"door_h":1,"door_colour":"#c8ccd0"}'),
  ('shop_glass','box','mat.smart_glass',0,'','{"y":1.15,"z":-0.85,"w":3.32,"d":1.82,"h":0.16,"far":0}'),
  ('shop_band','box','mat.white_composite',1,'','{"y":1.45,"z":-0.85,"w":3.36,"d":1.86,"h":0.26}'),
  ('shop_roof','flat_roof','mat.white_composite',0,'','{"y":1.71,"z":-0.85,"w":3.36,"d":1.86,"t":0.04,"overhang":0}'),
  ('door_glow','box','mat.light_strip',0,'','{"y":1.1,"z":0.06,"w":1.3,"d":0.03,"h":0.03,"far":0}'),
  ('roof_stripe','box','mat.white_composite',1,'','{"y":1.75,"z":-0.1,"w":3.36,"d":0.34,"h":0.02}'),
  ('roof_domes','dome','mat.white_composite',0,'','{"x":-0.9,"y":1.75,"z":-1,"w":0.6,"h":0.3,"segments":8,"repeat":2,"dx":1.8,"far":0}'),
  ('roof_solar','box','mat.solar_panel',0,'','{"y":1.78,"z":-0.85,"w":1,"d":1.4,"h":0.03,"pitch":14,"far":0}'),
  ('gantry_legs','box','mat.white_composite',0,'','{"x":-1.4,"y":0.05,"z":0.75,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8}'),
  ('gantry_legs_s','box','mat.white_composite',0,'','{"x":-1.4,"y":0.05,"z":1.45,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8,"far":0}'),
  ('gantry_beam','box','mat.white_composite',1,'','{"y":1.95,"z":1.1,"w":3,"d":0.22,"h":0.22}'),
  ('gantry_ends','box','mat.white_composite',0,'','{"x":-1.4,"y":1.95,"z":1.1,"w":0.14,"d":0.82,"h":0.12,"repeat":2,"dx":2.8,"far":0}'),
  ('trolley','box','mat.graphite',0,'','{"x":0.55,"y":1.79,"z":1.1,"w":0.26,"d":0.26,"h":0.16,"far":0}'),
  ('chain','cylinder','mat.steel',0,'','{"x":0.55,"y":1.05,"z":1.1,"w":0.03,"h":0.76,"segments":3,"far":0}'),
  ('rail_slung','box','mat.graphite',0,'','{"x":0.55,"y":0.92,"z":1,"w":0.16,"d":1.3,"h":0.14}'),
  ('sled','box','mat.white_composite',0,'','{"x":-0.69,"y":0.2,"z":0.95,"w":0.62,"d":1,"h":0.18}'),
  ('sled_glow','box','mat.light_strip',0,'','{"x":-0.69,"y":0.08,"z":0.95,"w":0.56,"d":0.9,"h":0.12,"far":0}'),
  ('cradle','box','mat.white_composite',0,'','{"x":-0.69,"y":0.38,"z":0.85,"w":0.4,"d":0.5,"h":0.22,"taper":0.2}'),
  ('gun','box','mat.graphite',0,'','{"x":-0.69,"y":0.5,"z":0.65,"w":0.12,"d":0.12,"h":1.3,"pitch":80}'),
  ('gun_coils','box','mat.white_composite',1,'','{"x":-0.69,"y":0.55,"z":1.2,"w":0.18,"d":0.2,"h":0.18,"far":0}'),
  ('crates','box','mat.white_composite',0,'','{"x":1,"y":0.05,"z":0.3,"w":0.3,"d":0.22,"h":0.18,"repeat":3,"dx":-0.34,"tint":"#c8d0d4","far":0}'),
  ('crates_2','box','mat.white_composite',0,'','{"x":0.83,"y":0.23,"z":0.3,"w":0.3,"d":0.22,"h":0.18,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"y":0.05,"z":1.6,"h":2.8,"fw":0.55,"fh":0.9,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.siege_workshop_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.siege_workshop'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.siege_workshop_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
