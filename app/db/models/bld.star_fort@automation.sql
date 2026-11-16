-- bld.star_fort@automation: Future star fort (mesh.star_fort_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.star_fort_automation','Future star fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.star_fort_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ramparts','custom','mat.white_composite',0,'','{"file":"bld.star_fort@automation/ramparts.mesh"}'),
  ('parade','plinth','mat.paving',0,'','{"w":2.4,"d":2.4,"h":0.03,"steps":1}'),
  ('gate','box','mat.white_composite',0,'','{"z":1.55,"w":0.95,"d":0.5,"h":1,"door":0.45,"door_h":0.7,"door_colour":"#2c3a40"}'),
  ('gate_band','box','mat.white_composite',1,'','{"y":0.82,"z":1.55,"w":0.99,"d":0.54,"h":0.12}'),
  ('barracks','box','mat.composite_panel',0,'','{"z":-0.55,"w":2,"d":0.7,"h":0.72,"door":0.3,"door_h":0.5,"door_colour":"#2c3a40"}'),
  ('barracks_glass','box','mat.smart_glass',0,'','{"y":0.4,"z":-0.55,"w":2.02,"d":0.72,"h":0.16,"far":0}'),
  ('barracks_band','box','mat.white_composite',1,'','{"y":0.72,"z":-0.55,"w":2.04,"d":0.74,"h":0.14}'),
  ('barracks_roof','flat_roof','mat.white_composite',0,'','{"y":0.86,"z":-0.55,"w":2.04,"d":0.74,"t":0.04,"overhang":0,"far":0}'),
  ('magazine','dome','mat.white_composite',0,'','{"x":-0.75,"z":0.55,"w":0.7,"h":0.4,"segments":8,"far":0}'),
  ('mast','banner','mat.cloth',1,'','{"x":0.7,"y":0,"z":0.45,"h":3.1,"fw":0.95,"fh":0.62,"pole_colour":"#e8ecec"}'),
  ('sensor','dome','mat.white_composite',0,'','{"x":-0.6,"y":0.9,"z":-0.55,"w":0.3,"h":0.2,"segments":8,"far":0}'),
  ('turret_se','dome','mat.white_composite',0,'','{"x":1.95,"y":0.76,"z":1.95,"yaw":45,"w":0.5,"h":0.26,"segments":8}'),
  ('gun_se','cylinder','mat.graphite',0,'turret_se','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84,"far":0}'),
  ('turret_ne','dome','mat.white_composite',0,'','{"x":1.95,"y":0.76,"z":-1.95,"yaw":135,"w":0.5,"h":0.26,"segments":8,"far":0}'),
  ('gun_ne','cylinder','mat.graphite',0,'turret_ne','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84,"far":0}'),
  ('turret_nw','dome','mat.white_composite',0,'','{"x":-1.95,"y":0.76,"z":-1.95,"yaw":225,"w":0.5,"h":0.26,"segments":8,"far":0}'),
  ('gun_nw','cylinder','mat.graphite',0,'turret_nw','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84,"far":0}'),
  ('turret_sw','dome','mat.white_composite',0,'','{"x":-1.95,"y":0.76,"z":1.95,"yaw":-45,"w":0.5,"h":0.26,"segments":8}'),
  ('gun_sw','cylinder','mat.graphite',0,'turret_sw','{"y":0.12,"z":0.1,"w":0.07,"h":0.6,"segments":5,"pitch":84,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.star_fort_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.star_fort'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.star_fort_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
