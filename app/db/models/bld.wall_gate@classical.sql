-- bld.wall_gate@classical: Classical gate (mesh.wall_gate_classical)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.wall_gate_classical','Classical gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.wall_gate_classical';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('arch','arch','mat.ashlar',0,'','{"w":1.0,"h":2.3,"t":0.7,"span":0.62,"spring":1.35,"segments":8}'),
  ('entablature','box','mat.limestone',0,'','{"y":2.3,"w":1.0,"d":0.78,"h":0.16}'),
  ('parapet','crenellation','mat.limestone',0,'','{"y":2.46,"z":0.27,"w":1.0,"h":0.3,"t":0.14,"merlon":0.25}'),
  ('doors','box','mat.bronze',0,'','{"z":-0.2,"w":0.6,"d":0.05,"h":1.3,"far":0}'),
  ('banner_l','banner','mat.cloth',1,'','{"x":-0.4,"y":2.46,"z":-0.1,"h":0.8,"fw":0.3,"fh":0.4}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.wall_gate_classical'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.wall_gate'), NULL, (SELECT id FROM def WHERE key='age.classical'), (SELECT id FROM def WHERE key='mesh.wall_gate_classical'), (SELECT id FROM def WHERE key='mat.limestone'), 100);
