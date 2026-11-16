-- bld.bastion@industrial: Industrial bastion (mesh.bastion_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bastion_industrial','Industrial bastion');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bastion_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('works','custom','mat.red_brick',0,'','{"file":"bld.bastion@industrial/works.mesh"}'),
  ('gorge','box','mat.red_brick',0,'','{"z":-0.88,"yaw":180,"w":1,"d":0.06,"h":0.9,"door":0.3,"door_h":0.6,"door_colour":"#2a221c"}'),
  ('cupola','dome','mat.cast_iron',1,'','{"y":0.82,"z":0.15,"w":0.7,"h":0.36,"segments":10}'),
  ('cupola_ring','cylinder','mat.cast_iron',0,'','{"y":0.8,"z":0.15,"w":0.78,"h":0.06,"segments":10,"far":0}'),
  ('gun','cylinder','mat.cast_iron',0,'','{"y":0.98,"z":0.4,"w":0.11,"h":0.55,"segments":6,"pitch":86}'),
  ('vent','cylinder','mat.cast_iron',0,'','{"y":1.15,"z":0.05,"w":0.08,"h":0.12,"segments":5,"far":0}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.45,"y":0.82,"z":-0.55,"h":1.8,"fw":0.8,"fh":0.56}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bastion_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bastion'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.bastion_industrial'), (SELECT id FROM def WHERE key='mat.red_brick'), 100);
