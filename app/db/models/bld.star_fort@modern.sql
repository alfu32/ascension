-- bld.star_fort@modern: Modern star fort (mesh.star_fort_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.star_fort_modern','Modern star fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.star_fort_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ramparts','custom','mat.concrete',0,'','{"file":"bld.star_fort@modern/ramparts.mesh"}'),
  ('parade','plinth','mat.concrete',0,'','{"w":2.4,"d":2.4,"h":0.03,"steps":1,"tint":"#b4b0a6"}'),
  ('gate','box','mat.concrete',0,'','{"z":1.55,"w":0.95,"d":0.5,"h":1,"door":0.45,"door_h":0.7,"door_colour":"#3a3e44"}'),
  ('gate_band','box','mat.cloth',1,'','{"y":0.82,"z":1.55,"w":0.99,"d":0.54,"h":0.12}'),
  ('barracks','box','mat.precast',0,'','{"z":-0.55,"w":2,"d":0.7,"h":0.75,"door":0.3,"door_h":0.5,"door_colour":"#3a4650","windows":1,"window_w":1.5,"window_h":0.16,"window_y":0.5,"opening":"#2b3b49"}'),
  ('barracks_fascia','box','mat.cloth',1,'','{"y":0.75,"z":-0.55,"w":2.04,"d":0.74,"h":0.12}'),
  ('barracks_roof','flat_roof','mat.tar',0,'','{"y":0.87,"z":-0.55,"w":2.04,"d":0.74,"t":0.04,"overhang":0,"far":0}'),
  ('magazine','dome','mat.turf',0,'','{"x":-0.75,"z":0.55,"w":0.7,"h":0.4,"segments":8,"far":0}'),
  ('mast','banner','mat.cloth',1,'','{"x":0.7,"z":0.45,"h":3.1,"fw":0.95,"fh":0.62}'),
  ('searchlight','cylinder','mat.white_metal',0,'','{"x":-0.6,"y":0.87,"z":-0.55,"w":0.26,"h":0.24,"segments":8,"far":0}'),
  ('turret_se','box','mat.olive_drab',0,'','{"x":1.95,"y":0.76,"z":1.95,"yaw":45,"w":0.48,"d":0.48,"h":0.22,"taper":0.25}'),
  ('turret_ne','box','mat.olive_drab',0,'','{"x":1.95,"y":0.76,"z":-1.95,"yaw":135,"w":0.48,"d":0.48,"h":0.22,"taper":0.25,"far":0}'),
  ('turret_nw','box','mat.olive_drab',0,'','{"x":-1.95,"y":0.76,"z":-1.95,"yaw":225,"w":0.48,"d":0.48,"h":0.22,"taper":0.25,"far":0}'),
  ('turret_sw','box','mat.olive_drab',0,'','{"x":-1.95,"y":0.76,"z":1.95,"yaw":-45,"w":0.48,"d":0.48,"h":0.22,"taper":0.25}'),
  ('gun_se','cylinder','mat.cast_iron',0,'turret_se','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84}'),
  ('gun_ne','cylinder','mat.cast_iron',0,'turret_ne','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84}'),
  ('gun_nw','cylinder','mat.cast_iron',0,'turret_nw','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84}'),
  ('gun_sw','cylinder','mat.cast_iron',0,'turret_sw','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.star_fort_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.star_fort'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.star_fort_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
