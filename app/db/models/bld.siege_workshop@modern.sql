-- bld.siege_workshop@modern: Modern siege workshop (mesh.siege_workshop_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.siege_workshop_modern','Modern siege workshop');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.siege_workshop_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('shop','box','mat.cladding',0,'','{"y":0.05,"z":-0.85,"w":3.3,"d":1.8,"h":1.4,"door":1.2,"door_h":1.05,"door_colour":"#c8ccd0","windows":1,"window_w":2.8,"window_h":0.18,"window_y":1.12,"side_windows":0,"opening":"#2b3b49","tint":"#a8b4a0"}'),
  ('shop_fascia','box','mat.cloth',1,'','{"y":1.45,"z":-0.85,"w":3.36,"d":1.86,"h":0.3}'),
  ('shop_roof','flat_roof','mat.tar',0,'','{"y":1.75,"z":-0.85,"w":3.36,"d":1.86,"t":0.04,"overhang":0}'),
  ('roof_stripe','box','mat.cloth',1,'','{"y":1.79,"z":-0.85,"w":3.36,"d":0.34,"h":0.02}'),
  ('roof_plant','box','mat.white_metal',0,'','{"x":1,"y":1.79,"z":-1.1,"w":0.6,"d":0.5,"h":0.25,"far":0}'),
  ('gantry_legs','box','mat.steel',0,'','{"x":-1.4,"y":0.05,"z":0.75,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8}'),
  ('gantry_legs_s','box','mat.steel',0,'','{"x":-1.4,"y":0.05,"z":1.45,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8,"far":0}'),
  ('gantry_beam','box','mat.cloth',1,'','{"y":1.95,"z":1.1,"w":3,"d":0.2,"h":0.2}'),
  ('gantry_ends','box','mat.steel',0,'','{"x":-1.4,"y":1.95,"z":1.1,"w":0.14,"d":0.82,"h":0.12,"repeat":2,"dx":2.8,"far":0}'),
  ('trolley','box','mat.steel',0,'','{"x":0.55,"y":1.81,"z":1.1,"w":0.25,"d":0.25,"h":0.14,"far":0}'),
  ('chain','cylinder','mat.steel',0,'','{"x":0.55,"y":1.05,"z":1.1,"w":0.03,"h":0.76,"segments":3,"far":0}'),
  ('barrel_slung','cylinder','mat.olive_drab',0,'','{"x":0.55,"y":0.95,"z":0.5,"w":0.13,"h":1.3,"segments":7,"pitch":90,"taper":0.2}'),
  ('gun_wheels','cylinder','mat.rubber',0,'','{"x":-1.05,"y":0.27,"z":0.75,"w":0.44,"h":0.12,"segments":10,"roll":90,"repeat":2,"dx":0.6}'),
  ('gun_cradle','box','mat.olive_drab',0,'','{"x":-0.69,"y":0.25,"z":0.75,"w":0.36,"d":0.5,"h":0.22}'),
  ('gun_barrel','cylinder','mat.olive_drab',0,'','{"x":-0.69,"y":0.4,"z":0.6,"w":0.09,"h":1.3,"taper":0.2,"segments":7,"pitch":78}'),
  ('gun_brake','box','mat.olive_drab',0,'','{"x":-0.69,"y":0.65,"z":1.85,"w":0.14,"d":0.14,"h":0.14,"far":0}'),
  ('gun_trails','box','mat.olive_drab',0,'','{"x":-0.85,"y":0.05,"z":0.5,"w":0.08,"d":0.08,"h":0.8,"pitch":-78,"repeat":2,"dx":0.32}'),
  ('crates','box','mat.olive_drab',0,'','{"x":1,"y":0.05,"z":0.3,"w":0.3,"d":0.22,"h":0.18,"repeat":3,"dx":-0.34,"far":0}'),
  ('crates_2','box','mat.olive_drab',0,'','{"x":0.83,"y":0.23,"z":0.3,"w":0.3,"d":0.22,"h":0.18,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"y":0.05,"z":1.6,"h":2.8,"fw":0.55,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.siege_workshop_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.siege_workshop'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.siege_workshop_modern'), (SELECT id FROM def WHERE key='mat.cladding'), 100);
