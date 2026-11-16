-- bld.bastion@modern: Modern bastion (mesh.bastion_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bastion_modern','Modern bastion');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bastion_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('works','custom','mat.concrete',0,'','{"file":"bld.bastion@modern/works.mesh"}'),
  ('gorge','box','mat.concrete',0,'','{"z":-0.88,"yaw":180,"w":1,"d":0.06,"h":0.9,"door":0.3,"door_h":0.6,"door_colour":"#3a3e44"}'),
  ('casemate','box','mat.concrete',0,'','{"y":0.82,"z":0.1,"w":0.78,"d":0.66,"h":0.3,"taper":0.22,"windows":1,"window_w":0.4,"window_h":0.07,"window_y":0.12,"opening":"#15181b"}'),
  ('casemate_band','box','mat.cloth',1,'','{"y":0.82,"z":0.1,"w":0.82,"d":0.7,"h":0.1}'),
  ('turret','dome','mat.olive_drab',0,'','{"y":1.12,"z":0.05,"w":0.42,"h":0.2,"segments":8}'),
  ('gun','cylinder','mat.cast_iron',0,'turret','{"y":0.1,"z":0.1,"w":0.08,"h":0.6,"segments":5,"pitch":86}'),
  ('flag','banner','mat.cloth',1,'','{"x":0.45,"y":0.82,"z":-0.55,"h":1.8,"fw":0.8,"fh":0.56}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bastion_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.bastion'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.bastion_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
