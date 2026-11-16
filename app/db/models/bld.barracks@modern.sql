-- bld.barracks@modern: Modern barracks (mesh.barracks_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.barracks_modern','Modern barracks');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.barracks_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.concrete',0,'','{"w":3.6,"d":3.6,"h":0.05,"steps":1,"tint":"#9c9890"}'),
  ('walls','wall','mat.concrete',0,'','{"w":3.5,"d":3.5,"h":0.75,"t":0.16,"ring":1,"gap":1.2}'),
  ('wire','fence','mat.steel',0,'','{"y":0.75,"w":3.5,"d":3.5,"h":0.22,"spacing":0.6,"rails":2,"ring":1,"gap":1.2,"far":0}'),
  ('gate_piers','box','mat.concrete',0,'','{"x":-0.7,"z":1.67,"w":0.26,"d":0.26,"h":1.05,"repeat":2,"dx":1.4}'),
  ('boom','box','mat.cloth',1,'','{"x":-0.05,"y":0.7,"z":1.67,"w":1.2,"d":0.06,"h":0.07}'),
  ('guard_hut','box','mat.concrete',0,'','{"x":-1.1,"y":0.05,"z":1.2,"w":0.5,"d":0.45,"h":0.7,"windows":1,"side_windows":1,"window_w":0.3,"window_h":0.2,"window_y":0.38,"opening":"#2b3b49"}'),
  ('guard_roof','flat_roof','mat.cloth',1,'','{"x":-1.1,"y":0.75,"z":1.2,"w":0.5,"d":0.45,"t":0.06,"overhang":0.05}'),
  ('block','box','mat.precast',0,'','{"y":0.05,"z":-0.95,"w":3,"d":1.2,"h":1.55,"door":0.42,"door_h":0.6,"door_x":-1.15,"door_colour":"#3a4650","windows":1,"window_w":1.7,"window_h":0.24,"window_y":0.32,"window_rows":2,"row_gap":0.65,"side_windows":1,"opening":"#2b3b49"}'),
  ('block_fascia','box','mat.cloth',1,'','{"y":1.54,"z":-0.95,"w":3.06,"d":1.26,"h":0.2}'),
  ('block_roof','flat_roof','mat.tar',0,'','{"y":1.74,"z":-0.95,"w":3.06,"d":1.26,"t":0.04,"overhang":0}'),
  ('antenna','cylinder','mat.steel',0,'','{"x":1.2,"y":1.78,"z":-1.2,"w":0.04,"h":1.4,"segments":4}'),
  ('antenna_light','box','mat.signal_red',0,'','{"x":1.2,"y":3.18,"z":-1.2,"w":0.08,"d":0.08,"h":0.08,"far":0}'),
  ('annex','box','mat.precast',0,'','{"x":1.15,"y":0.05,"z":0.4,"yaw":-90,"w":1.2,"d":0.8,"h":1,"windows":1,"window_w":0.9,"window_h":0.22,"window_y":0.4,"opening":"#2b3b49"}'),
  ('annex_roof','flat_roof','mat.cloth',1,'','{"x":1.15,"y":1.05,"z":0.4,"w":0.8,"d":1.2,"t":0.04,"overhang":0.04}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.55,"y":0.05,"z":0.35,"h":4,"fw":0.9,"fh":0.6}'),
  ('jeep','box','mat.olive_drab',0,'','{"x":0.15,"y":0.12,"z":0.55,"w":0.32,"d":0.6,"h":0.16,"far":0}'),
  ('jeep_wheels','cylinder','mat.rubber',0,'','{"x":-0.03,"y":0.11,"z":0.35,"w":0.2,"h":0.36,"segments":6,"roll":90,"repeat":2,"dz":0.42,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.barracks_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.barracks'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.barracks_modern'), (SELECT id FROM def WHERE key='mat.precast'), 100);
