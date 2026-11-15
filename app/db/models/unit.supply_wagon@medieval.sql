-- unit.supply_wagon@medieval: Supply wagon (mesh.supply_wagon_medieval)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.supply_wagon_medieval','Supply wagon');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.supply_wagon_medieval';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('ox_w','quadruped','mat.hide',0,'','{"x":-0.24,"z":0.82,"h":0.82,"l":1,"w":0.36,"pace":1.1,"stride":20,"tint":"#9a8270"}'),
  ('ox_e','quadruped','mat.hide',0,'','{"x":0.24,"z":0.82,"h":0.82,"l":1,"w":0.36,"pace":1.1,"stride":20,"tint":"#9a8270","far":0}'),
  ('yoke','box','mat.oak',0,'','{"y":0.72,"z":1.12,"w":0.86,"d":0.08,"h":0.08,"far":0}'),
  ('pole','box','mat.oak',0,'','{"y":0.42,"z":0.3,"w":0.06,"d":0.06,"h":0.95,"pitch":82,"far":0}'),
  ('bed','box','mat.sawn_timber',0,'','{"y":0.3,"z":-0.55,"w":0.78,"d":1.25,"h":0.28}'),
  ('wheels_w','wheel','mat.oak',0,'','{"x":-0.44,"z":-0.12,"w":0.5,"t":0.07,"spokes":1,"segments":8,"repeat":2,"dz":-0.9}'),
  ('wheels_e','wheel','mat.oak',0,'','{"x":0.44,"z":-0.12,"w":0.5,"t":0.07,"spokes":1,"segments":8,"repeat":2,"dz":-0.9}'),
  ('cover','cylinder','mat.canvas',0,'','{"y":0.62,"z":-1.15,"w":0.76,"h":1.1,"segments":10,"pitch":90,"bottom":1}'),
  ('bands','cylinder','mat.cloth',1,'','{"y":0.62,"z":-0.95,"w":0.8,"h":0.22,"segments":10,"pitch":90,"bottom":0,"repeat":2,"dz":0.5,"far":0}'),
  ('seat','box','mat.oak',0,'','{"y":0.58,"z":-0.02,"w":0.6,"d":0.22,"h":0.12,"far":0}'),
  ('driver','figure','mat.cloth',1,'','{"y":0.3,"z":0,"seat":1,"build":1.05,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.supply_wagon_medieval'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.supply_wagon'), NULL, (SELECT id FROM def WHERE key='age.medieval'), (SELECT id FROM def WHERE key='mesh.supply_wagon_medieval'), (SELECT id FROM def WHERE key='mat.oak'), 100);
