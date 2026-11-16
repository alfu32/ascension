-- bld.bastion@enlightenment: Bastion (mesh.bastion_enlightenment)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bastion_enlightenment','Bastion');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bastion_enlightenment';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('works','custom','mat.ashlar',0,'','{"file":"bld.bastion@enlightenment/works.mesh"}'),
  ('gorge','box','mat.brick',0,'','{"z":-0.88,"yaw":180,"w":1,"d":0.06,"h":0.9,"door":0.3,"door_h":0.6,"door_colour":"#2a221c"}'),
  ('guerite','cylinder','mat.ashlar',0,'','{"y":0.86,"z":0.78,"w":0.24,"h":0.34,"segments":8}'),
  ('guerite_cap','dome','mat.cloth',1,'','{"y":1.2,"z":0.78,"w":0.3,"h":0.2,"segments":8}'),
  ('guns','cylinder','mat.iron',0,'','{"x":-0.06,"y":0.97,"z":0.34,"w":0.1,"h":0.4,"segments":6,"yaw":-52,"pitch":88,"tint":"#4a4e56"}'),
  ('guns_e','cylinder','mat.iron',0,'','{"x":0.06,"y":0.97,"z":0.34,"w":0.1,"h":0.4,"segments":6,"yaw":52,"pitch":88,"tint":"#4a4e56"}'),
  ('carriage','box','mat.oak',0,'','{"x":-0.16,"y":0.9,"z":0.42,"yaw":-52,"w":0.2,"d":0.32,"h":0.07,"far":0}'),
  ('carriage_e','box','mat.oak',0,'','{"x":0.16,"y":0.9,"z":0.42,"yaw":52,"w":0.2,"d":0.32,"h":0.07,"far":0}'),
  ('shot','dome','mat.iron',0,'','{"x":0,"y":0.9,"z":-0.35,"w":0.26,"h":0.14,"segments":6,"tint":"#5a5e66","far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.4,"y":0.9,"z":-0.55,"h":1.8,"fw":0.8,"fh":0.56}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bastion_enlightenment'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bastion'), NULL, (SELECT id FROM def WHERE key='age.enlightenment'), (SELECT id FROM def WHERE key='mesh.bastion_enlightenment'), (SELECT id FROM def WHERE key='mat.ashlar'), 100);
