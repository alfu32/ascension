-- bld.palisade_gate@ancient: Palisade gate (mesh.palisade_gate_ancient)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.palisade_gate_ancient','Palisade gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.palisade_gate_ancient';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.packed_earth',0,'','{"w":2.96,"d":0.86,"h":0.02}'),
  ('post_l','cylinder','mat.timber',0,'','{"x":-1.4,"w":0.18,"h":1.95,"segments":6}'),
  ('post_r','cylinder','mat.timber',0,'','{"x":1.4,"w":0.18,"h":1.95,"segments":6}'),
  ('lintel','box','mat.sawn_timber',0,'','{"y":1.7,"w":2.96,"d":0.18,"h":0.14}'),
  ('lintel_band','box','mat.cloth',1,'','{"y":1.69,"w":0.96,"d":0.2,"h":0.16}'),
  ('leaf_l','box','mat.sawn_timber',0,'','{"x":-1.21,"z":0.2,"yaw":-70,"w":0.5,"d":0.06,"h":1.25}'),
  ('leaf_r','box','mat.sawn_timber',0,'','{"x":1.21,"z":0.2,"yaw":70,"w":0.5,"d":0.06,"h":1.25}'),
  ('banner','banner','mat.cloth',1,'','{"x":0.07,"y":1.95,"h":0.8,"fw":0.42,"fh":0.36}'),
  ('gate_shield','shield','mat.hide',0,'','{"y":1.62,"z":0.1,"w":0.26,"t":0.04,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.palisade_gate_ancient'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.palisade_gate'), NULL, (SELECT id FROM def WHERE key='age.ancient'), (SELECT id FROM def WHERE key='mesh.palisade_gate_ancient'), (SELECT id FROM def WHERE key='mat.timber'), 100);
