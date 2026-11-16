-- bld.dock@modern: Modern dock (mesh.dock_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.dock_modern','Modern dock');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.dock_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":3.6,"d":3.6,"h":0.04,"steps":1,"tint":"#a8a49c"}'),
  ('basin','box','mat.white_metal',0,'','{"x":-0.85,"y":0.04,"z":0,"w":1.9,"d":3.5,"h":0.005,"tint":"#6890b8"}'),
  ('quay','box','mat.concrete',0,'','{"x":0.95,"w":1.6,"d":3.5,"h":0.42,"tint":"#c8c4ba"}'),
  ('quay_edge','box','mat.concrete',0,'','{"x":0.2,"y":0.42,"w":0.1,"d":3.5,"h":0.04,"tint":"#f0ece0","far":0}'),
  ('bollards','cylinder','mat.cast_iron',0,'','{"x":0.3,"y":0.42,"z":-0.9,"w":0.1,"h":0.14,"segments":6,"repeat":3,"dz":1,"far":0}'),
  ('ship','hull','mat.iron',0,'','{"x":-0.85,"y":0.12,"z":0.3,"yaw":90,"w":2.6,"d":0.75,"h":0.42,"sheer":0.08,"segments":6,"tint":"#5a6a7a"}'),
  ('deck','box','mat.cast_iron',0,'ship','{"y":0.38,"w":2.3,"d":0.62,"h":0.04}'),
  ('bridge','box','mat.white_metal',0,'ship','{"x":-0.95,"y":0.42,"w":0.32,"d":0.66,"h":0.55,"side_windows":1,"window_w":0.5,"window_h":0.08,"window_y":0.42,"opening":"#2b3b49"}'),
  ('funnel','box','mat.cloth',1,'ship','{"x":-1.08,"y":0.97,"w":0.18,"d":0.24,"h":0.26}'),
  ('containers','box','mat.cladding',0,'ship','{"x":-0.45,"y":0.42,"w":0.45,"d":0.6,"h":0.2,"repeat":3,"dx":0.5,"tint":"#c05038"}'),
  ('containers_2','box','mat.cladding',0,'ship','{"x":-0.2,"y":0.62,"w":0.45,"d":0.6,"h":0.2,"repeat":2,"dx":0.5,"tint":"#3a6aa0","far":0}'),
  ('containers_3','box','mat.cladding',0,'ship','{"x":0.8,"y":0.62,"w":0.45,"d":0.6,"h":0.2,"tint":"#d8d0b0"}'),
  ('warehouse','box','mat.cladding',0,'','{"x":0.95,"y":0.42,"z":-1.3,"w":1.5,"d":0.85,"h":0.95,"door":0.6,"door_h":0.7,"door_colour":"#c8ccd0"}'),
  ('warehouse_fascia','box','mat.cloth',1,'','{"x":0.95,"y":1.37,"z":-1.3,"w":1.56,"d":0.91,"h":0.12}'),
  ('warehouse_roof','flat_roof','mat.cloth',1,'','{"x":0.95,"y":1.49,"z":-1.3,"w":1.56,"d":0.91,"t":0.04,"overhang":0}'),
  ('crane_legs','box','mat.steel',0,'','{"x":0.4,"y":0.42,"z":-0.55,"w":0.12,"d":0.12,"h":2.4,"repeat":2,"dx":1}'),
  ('crane_legs_s','box','mat.steel',0,'','{"x":0.4,"y":0.42,"z":0.05,"w":0.12,"d":0.12,"h":2.4,"repeat":2,"dx":1}'),
  ('crane_sills','box','mat.steel',0,'','{"x":0.4,"y":0.42,"z":-0.25,"w":0.16,"d":0.76,"h":0.12,"repeat":2,"dx":1,"far":0}'),
  ('crane_ties','box','mat.steel',0,'','{"x":0.9,"y":1.7,"z":-0.55,"w":1.1,"d":0.08,"h":0.1,"repeat":2,"dz":0.6,"far":0}'),
  ('boom','box','mat.cloth',1,'','{"x":0.1,"y":2.82,"z":-0.25,"w":3.3,"d":0.24,"h":0.2}'),
  ('machinery','box','mat.white_metal',0,'','{"x":1.2,"y":3.02,"z":-0.25,"w":0.55,"d":0.42,"h":0.3}'),
  ('apex','box','mat.steel',0,'','{"x":0.9,"y":3.02,"z":-0.25,"w":0.08,"d":0.08,"h":0.5,"far":0}'),
  ('trolley','box','mat.steel',0,'','{"x":-0.85,"y":2.68,"z":-0.25,"w":0.3,"d":0.3,"h":0.14}'),
  ('cables','cylinder','mat.steel',0,'','{"x":-0.85,"y":1.86,"z":-0.25,"w":0.03,"h":0.82,"segments":3,"far":0}'),
  ('load','box','mat.cladding',0,'','{"x":-0.85,"y":1.66,"z":-0.25,"w":0.6,"d":0.25,"h":0.2,"tint":"#d8a030","far":0}'),
  ('stack','box','mat.cladding',0,'','{"x":0.75,"y":0.42,"z":0.95,"w":0.25,"d":0.6,"h":0.22,"repeat":3,"dx":0.3,"tint":"#c87838"}'),
  ('stack_2','box','mat.cladding',0,'','{"x":0.9,"y":0.64,"z":0.95,"w":0.25,"d":0.6,"h":0.22,"repeat":2,"dx":0.3,"tint":"#4a8a6a","far":0}'),
  ('light_mast','cylinder','mat.steel',0,'','{"x":1.6,"y":0.42,"z":1.55,"w":0.06,"h":2.2,"segments":4,"far":0}'),
  ('light_head','box','mat.white_metal',0,'','{"x":1.6,"y":2.62,"z":1.55,"w":0.3,"d":0.12,"h":0.12,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.dock_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.dock'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.dock_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
