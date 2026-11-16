-- bld.tower@modern: Modern tower (mesh.tower_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.tower_modern','Modern tower');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.tower_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('base','box','mat.concrete',0,'','{"w":1.75,"d":1.75,"h":0.45,"taper":0.1}'),
  ('shaft','box','mat.concrete',0,'','{"y":0.45,"w":1.2,"d":1.2,"h":2.95,"door":0.36,"door_h":0.6,"door_colour":"#3a4650","windows":1,"side_windows":1,"window_w":0.32,"window_h":0.08,"window_y":1,"window_rows":2,"row_gap":0.9,"opening":"#1a1e22"}'),
  ('band','box','mat.cloth',1,'','{"y":3.1,"w":1.26,"d":1.26,"h":0.3}'),
  ('cab_floor','box','mat.concrete',0,'','{"y":3.4,"w":1.75,"d":1.75,"h":0.12}'),
  ('cab','box','mat.curtain_wall',0,'','{"y":3.52,"w":1.5,"d":1.5,"h":0.45}'),
  ('cab_roof','box','mat.concrete',0,'','{"y":3.97,"w":1.75,"d":1.75,"h":0.1}'),
  ('roof_panel','box','mat.cloth',1,'','{"y":4.07,"w":1.1,"d":1.1,"h":0.02}'),
  ('merlons','crenellation','mat.concrete',0,'','{"y":4.07,"w":1.75,"d":1.75,"h":0.24,"t":0.12,"merlon":0.3,"ring":1}'),
  ('searchlight','cylinder','mat.white_metal',0,'','{"x":-0.45,"y":4.07,"z":0.45,"w":0.3,"h":0.28,"segments":8}'),
  ('lamp','cylinder','mat.glow',0,'','{"x":-0.45,"y":4.21,"z":0.56,"w":0.24,"h":0.04,"segments":8,"pitch":90,"far":0}'),
  ('mast','banner','mat.cloth',1,'','{"x":0.45,"y":4.07,"z":-0.45,"h":1.6,"fw":0.7,"fh":0.46}'),
  ('aerial','cylinder','mat.steel',0,'','{"x":-0.5,"y":4.07,"z":-0.5,"w":0.04,"h":1.2,"segments":4}'),
  ('aerial_light','box','mat.signal_red',0,'','{"x":-0.5,"y":5.27,"z":-0.5,"w":0.08,"d":0.08,"h":0.08,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.tower_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.tower'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.tower_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
