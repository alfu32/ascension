-- bld.star_fort@enlightenment: Star fort (mesh.star_fort_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.star_fort_enlightenment','Star fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.star_fort_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ramparts','custom','mat.ashlar',0,'','{"file":"bld.star_fort@enlightenment/ramparts.mesh"}'),
  ('parade','plinth','mat.packed_earth',0,'','{"w":2.4,"d":2.4,"h":0.03,"steps":1}'),
  ('gate','box','mat.ashlar',0,'','{"z":1.55,"w":0.95,"d":0.5,"h":1.05,"door":0.42,"door_h":0.68,"door_colour":"#2a221c"}'),
  ('gate_pediment','box','mat.ashlar',0,'','{"y":1.05,"z":1.55,"yaw":90,"w":0.5,"d":0.95,"h":0.02,"gable":26}'),
  ('gate_arms','shield','mat.cloth',1,'','{"y":0.86,"z":1.81,"w":0.26,"h":0.3,"t":0.03,"boss":0,"point":0.4,"rim":"#d6cbb0","far":0}'),
  ('barracks','box','mat.brick',0,'','{"z":-0.55,"w":2,"d":0.7,"h":0.72,"door":0.3,"door_h":0.5,"windows":5,"window_y":0.3,"window_w":0.12,"window_h":0.24,"opening":"#2c3642"}'),
  ('barracks_roof','hip_roof','mat.pantile',0,'','{"y":0.72,"z":-0.55,"w":2,"d":0.7,"pitch":40,"overhang":0.08,"t":0.06}'),
  ('barracks_cap','hip_roof','mat.cloth',1,'','{"y":0.966,"z":-0.55,"w":1.6,"d":0.3,"pitch":40,"overhang":0,"t":0.03}'),
  ('magazine','box','mat.ashlar',0,'','{"x":-0.75,"z":0.55,"w":0.6,"d":0.5,"h":0.5}'),
  ('magazine_roof','gable_roof','mat.slate',0,'','{"x":-0.75,"y":0.5,"z":0.55,"w":0.6,"d":0.5,"pitch":40,"overhang":0.05,"t":0.05}'),
  ('flagpole','banner','mat.cloth',1,'','{"x":0.6,"z":0.5,"h":2.6,"fw":0.95,"fh":0.62}'),
  ('gun_se','cylinder','mat.iron',0,'','{"x":1.95,"y":0.76,"z":1.95,"w":0.1,"h":0.45,"segments":6,"yaw":45,"pitch":86,"tint":"#4a4e56","far":0}'),
  ('gun_ne','cylinder','mat.iron',0,'','{"x":1.95,"y":0.76,"z":-1.95,"w":0.1,"h":0.45,"segments":6,"yaw":135,"pitch":86,"tint":"#4a4e56","far":0}'),
  ('gun_nw','cylinder','mat.iron',0,'','{"x":-1.95,"y":0.76,"z":-1.95,"w":0.1,"h":0.45,"segments":6,"yaw":225,"pitch":86,"tint":"#4a4e56","far":0}'),
  ('gun_sw','cylinder','mat.iron',0,'','{"x":-1.95,"y":0.76,"z":1.95,"w":0.1,"h":0.45,"segments":6,"yaw":-45,"pitch":86,"tint":"#4a4e56","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.star_fort_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.star_fort'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.star_fort_enlightenment'), (SELECT id FROM def WHERE key='mat.ashlar'), 100);
