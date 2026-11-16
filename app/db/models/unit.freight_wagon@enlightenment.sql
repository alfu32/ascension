-- unit.freight_wagon@enlightenment: Freight wagon (mesh.freight_wagon_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.freight_wagon_enlightenment','Freight wagon');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.freight_wagon_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('lead_w','quadruped','mat.hide',0,'','{"x":-0.24,"z":1.75,"h":0.8,"pace":0.8,"stride":24,"plain":1,"tint":"#c89870","far":0}'),
  ('lead_e','quadruped','mat.hide',0,'','{"x":0.24,"z":1.75,"h":0.8,"pace":0.8,"stride":24,"plain":1,"tint":"#e6c8a0","far":0}'),
  ('wheel_w','quadruped','mat.hide',0,'','{"x":-0.24,"z":0.72,"h":0.86,"pace":0.8,"stride":24,"plain":1,"tint":"#b07850"}'),
  ('wheel_e','quadruped','mat.hide',0,'','{"x":0.24,"z":0.72,"h":0.86,"pace":0.8,"stride":24,"plain":1,"tint":"#d4a878","far":0}'),
  ('pole','box','mat.oak',0,'','{"y":0.45,"z":-0.1,"w":0.06,"d":0.06,"h":2.1,"pitch":88,"far":0}'),
  ('bed','box','mat.cloth',1,'','{"y":0.42,"z":-0.95,"w":0.84,"d":1.7,"h":0.32,"taper":0.06}'),
  ('bed_rail','box','mat.oak',0,'','{"y":0.72,"z":-0.95,"w":0.88,"d":1.74,"h":0.05,"far":0}'),
  ('wheels_front','wheel','mat.oak',0,'','{"x":-0.48,"z":-0.4,"w":0.6,"t":0.07,"spokes":1,"segments":8,"repeat":2,"dx":0.96,"far":0}'),
  ('wheels_back','wheel','mat.oak',0,'','{"x":-0.5,"z":-1.5,"w":0.8,"t":0.08,"spokes":1,"segments":10,"repeat":2,"dx":1}'),
  ('cover','cylinder','mat.canvas',0,'','{"y":0.74,"z":-1.62,"w":0.92,"h":1.34,"segments":10,"pitch":90,"bottom":1}'),
  ('bonnet_back','cylinder','mat.canvas',0,'','{"y":0.74,"z":-1.62,"w":0.96,"h":0.3,"segments":10,"pitch":-76,"bottom":1,"far":0}'),
  ('bonnet_front','cylinder','mat.canvas',0,'','{"y":0.74,"z":-0.34,"w":0.96,"h":0.3,"segments":10,"pitch":76,"bottom":1,"far":0}'),
  ('bows','cylinder','mat.cloth',1,'','{"y":0.74,"z":-1.38,"w":0.95,"h":0.2,"segments":10,"pitch":90,"bottom":0,"repeat":3,"dz":0.36,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.freight_wagon_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.freight_wagon'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.freight_wagon_enlightenment'), (SELECT id FROM def WHERE key='mat.oak'), 100);
