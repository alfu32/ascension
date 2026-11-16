-- bld.siege_workshop@industrial: Industrial siege workshop (mesh.siege_workshop_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.siege_workshop_industrial','Industrial siege workshop');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.siege_workshop_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.cinders',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1}'),
  ('shop','box','mat.red_brick',0,'','{"z":-0.85,"w":3.3,"d":1.8,"h":1.35,"door":1.2,"door_h":1.05,"windows":4,"window_y":0.3,"window_w":0.2,"window_h":0.5,"side_windows":2,"opening":"#22282f","door_colour":"#2a2a2c"}'),
  ('roof_slopes','box','mat.slate',0,'','{"x":0,"y":1.35,"z":-1.75,"w":3.2,"d":0.05,"h":0.732,"pitch":55.01,"repeat":3,"dz":0.6}'),
  ('roof_glass','box','mat.glass',0,'','{"x":0,"y":1.35,"z":-1.17,"w":3.2,"d":0.04,"h":0.42,"repeat":3,"dz":0.6}'),
  ('parapets','box','mat.cloth',1,'','{"x":-1.6,"y":1.35,"z":-0.85,"w":0.12,"d":1.8,"h":0.45,"repeat":2,"dx":3.2}'),
  ('parapet_caps','box','mat.cloth',1,'','{"x":-1.6,"y":1.8,"z":-0.85,"w":0.16,"d":1.84,"h":0.08,"repeat":2,"dx":3.2}'),
  ('fascia','box','mat.cloth',1,'','{"y":1.25,"z":0.06,"w":3.32,"d":0.04,"h":0.22}'),
  ('gantry_legs','box','mat.cast_iron',0,'','{"x":-1.4,"z":0.75,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8}'),
  ('gantry_legs_s','box','mat.cast_iron',0,'','{"x":-1.4,"z":1.45,"w":0.12,"d":0.12,"h":1.9,"repeat":2,"dx":2.8,"far":0}'),
  ('gantry_beam','box','mat.cloth',1,'','{"y":1.9,"z":1.1,"w":3,"d":0.2,"h":0.2}'),
  ('gantry_ends','box','mat.cast_iron',0,'','{"x":-1.4,"y":1.9,"z":1.1,"w":0.14,"d":0.82,"h":0.12,"repeat":2,"dx":2.8,"far":0}'),
  ('trolley','box','mat.cast_iron',0,'','{"x":0.35,"y":1.76,"z":1.1,"w":0.25,"d":0.25,"h":0.14,"far":0}'),
  ('chain','cylinder','mat.steel',0,'','{"x":0.35,"y":1,"z":1.1,"w":0.03,"h":0.76,"segments":3,"far":0}'),
  ('barrel_slung','cylinder','mat.cast_iron',0,'','{"x":0.35,"y":0.9,"z":0.65,"w":0.16,"h":1.1,"segments":7,"pitch":90,"taper":0.25}'),
  ('gun_wheels','cylinder','mat.oak',0,'','{"x":-0.85,"y":0.36,"z":0.95,"w":0.72,"h":0.07,"segments":10,"roll":90,"repeat":2,"dx":0.7}'),
  ('gun_shield','box','mat.olive_drab',0,'','{"x":-0.5,"y":0.4,"z":1.1,"w":0.8,"d":0.04,"h":0.5}'),
  ('gun_barrel','cylinder','mat.cast_iron',0,'','{"x":-0.5,"y":0.55,"z":0.5,"w":0.1,"h":1.1,"taper":0.25,"segments":7,"pitch":86}'),
  ('gun_trail','box','mat.olive_drab',0,'','{"x":-0.5,"z":0.15,"w":0.16,"d":0.1,"h":0.8,"pitch":-70,"far":0}'),
  ('shells','cylinder','mat.brass',0,'','{"x":1.2,"z":0.5,"w":0.1,"h":0.3,"segments":5,"repeat":4,"dx":-0.13,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":1.6,"z":1.6,"h":2.8,"fw":0.55,"fh":0.9}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.siege_workshop_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.siege_workshop'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.siege_workshop_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
