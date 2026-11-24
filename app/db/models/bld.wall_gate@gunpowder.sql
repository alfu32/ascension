-- bld.wall_gate@gunpowder: Ravelin gate (mesh.wall_gate_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.wall_gate_gunpowder','Ravelin gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.wall_gate_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.packed_earth',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('scarp_l','box','mat.brick',0,'','{"x":-0.36,"w":0.22,"d":1.25,"h":1.6,"taper":0.2}'),
  ('scarp_r','box','mat.brick',0,'','{"x":0.36,"w":0.22,"d":1.25,"h":1.6,"taper":0.2}'),
  ('arch','arch','mat.limestone',0,'','{"y":0.0,"z":0.2,"w":0.98,"h":1.75,"t":0.36,"span":0.52,"spring":1.0,"segments":8}'),
  ('pediment','gable_roof','mat.cloth',1,'','{"y":1.75,"z":0.2,"w":0.84,"d":0.36,"pitch":22,"overhang":0.02,"t":0.06}'),
  ('doors','box','mat.oak',0,'','{"z":0.1,"w":0.52,"d":0.05,"h":1.0,"far":0}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.0,"y":1.85,"z":-0.1,"h":0.7,"fw":0.3,"fh":0.22}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.wall_gate_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.wall_gate'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.wall_gate_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
