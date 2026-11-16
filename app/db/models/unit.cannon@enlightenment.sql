-- unit.cannon@enlightenment: Cannon (mesh.cannon_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.cannon_enlightenment','Cannon');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.cannon_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('wheels','wheel','mat.oak',0,'','{"x":-0.4,"y":0,"z":0,"w":0.74,"t":0.07,"spokes":1,"segments":10,"repeat":2,"dx":0.8}'),
  ('axle','box','mat.iron',0,'','{"y":0.33,"w":0.82,"d":0.08,"h":0.08}'),
  ('cheeks','box','mat.cloth',1,'','{"x":-0.13,"y":0.24,"z":-0.1,"w":0.07,"d":0.75,"h":0.26,"repeat":2,"dx":0.26}'),
  ('trail','box','mat.oak',0,'','{"y":0,"z":-1.05,"w":0.24,"d":0.12,"h":1.05,"pitch":66}'),
  ('barrel','cylinder','mat.bronze',0,'','{"y":0.5,"z":-0.42,"w":0.22,"h":1.2,"taper":0.32,"segments":8,"pitch":87,"recoil":0.12}'),
  ('cascabel','dome','mat.bronze',0,'barrel','{"w":0.22,"h":0.1,"segments":8,"pitch":180,"far":0}'),
  ('swell','cylinder','mat.bronze',0,'barrel','{"y":1.08,"w":0.2,"h":0.12,"segments":8,"far":0}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":1.2,"w":0.08,"h":0.08,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.6,"w":0.62,"h":0.6,"segments":8,"pitch":180,"far":0}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.62,"z":-0.5,"hold_right":20,"far":0}'),
  ('hat','cylinder','mat.felt',0,'gunner.head','{"y":-0.04,"yaw":90,"w":0.4,"h":0.07,"segments":3,"far":0}'),
  ('rammer','cylinder','mat.timber',0,'gunner.right_hand','{"y":-0.3,"w":0.03,"h":1.05,"segments":4,"far":0}'),
  ('rammer_head','cylinder','mat.felt',0,'rammer','{"y":1.05,"w":0.08,"h":0.1,"segments":6,"far":0}'),
  ('shot','dome','mat.iron',0,'','{"x":0.5,"z":-0.65,"w":0.28,"h":0.14,"segments":6,"tint":"#5a5e66","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.cannon_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.cannon'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.cannon_enlightenment'), (SELECT id FROM def WHERE key='mat.oak'), 100);
