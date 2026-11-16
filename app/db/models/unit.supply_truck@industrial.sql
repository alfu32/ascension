-- unit.supply_truck@industrial: Supply truck (mesh.supply_truck_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.supply_truck_industrial','Supply truck');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.supply_truck_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels_front','wheel','mat.rubber',0,'','{"x":-0.47,"z":0.78,"w":0.56,"t":0.12,"spokes":2,"segments":10,"repeat":2,"dx":0.94}'),
  ('wheels_back','wheel','mat.rubber',0,'','{"x":-0.5,"z":-0.62,"w":0.6,"t":0.2,"spokes":2,"segments":10,"repeat":2,"dx":1}'),
  ('chassis','box','mat.cast_iron',0,'','{"y":0.22,"z":0,"w":0.66,"d":2.2,"h":0.1}'),
  ('bonnet','box','mat.olive_drab',0,'','{"y":0.32,"z":0.86,"w":0.56,"d":0.5,"h":0.34,"taper":0.1}'),
  ('radiator','box','mat.brass',0,'','{"y":0.32,"z":1.11,"w":0.5,"d":0.04,"h":0.36,"far":0}'),
  ('guards','box','mat.olive_drab',0,'','{"x":-0.47,"y":0.56,"z":0.78,"w":0.18,"d":0.6,"h":0.03,"repeat":2,"dx":0.94,"far":0}'),
  ('cab','box','mat.olive_drab',1,'','{"y":0.32,"z":0.42,"w":0.92,"d":0.42,"h":0.66,"windows":1,"window_y":0.36,"window_w":0.6,"window_h":0.22,"side_windows":1,"opening":"#22262a"}'),
  ('cab_roof','box','mat.cast_iron',0,'','{"y":0.98,"z":0.42,"w":0.96,"d":0.48,"h":0.04}'),
  ('tailboard','box','mat.cloth',1,'','{"y":0.32,"z":-1.22,"w":1.02,"d":0.04,"h":0.28}'),
  ('bed','box','mat.sawn_timber',0,'','{"y":0.32,"z":-0.5,"w":1,"d":1.42,"h":0.26}'),
  ('tilt','cylinder','mat.khaki',0,'','{"y":0.62,"z":-1.18,"w":0.98,"h":1.34,"segments":10,"pitch":90,"bottom":1}'),
  ('tilt_sides','box','mat.khaki',0,'','{"y":0.58,"z":-0.5,"w":0.98,"d":1.34,"h":0.06}'),
  ('hoops','cylinder','mat.cloth',1,'','{"y":0.62,"z":-0.98,"w":1,"h":0.06,"segments":10,"pitch":90,"bottom":0,"repeat":3,"dz":0.46,"far":0}'),
  ('crates','box','mat.timber',0,'','{"y":0.58,"z":-1.12,"w":0.7,"d":0.08,"h":0.26,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.supply_truck_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.supply_truck'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.supply_truck_industrial'), (SELECT id FROM def WHERE key='mat.olive_drab'), 100);
