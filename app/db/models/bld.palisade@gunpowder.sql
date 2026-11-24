-- bld.palisade@gunpowder: Rampart wall (mesh.palisade_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.palisade_gunpowder','Rampart wall');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.palisade_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.packed_earth',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('scarp','box','mat.brick',0,'','{"w":0.98,"d":1.25,"h":0.5}'),
  ('scarp_2','box','mat.brick',0,'','{"y":0.5,"w":0.98,"d":1.05,"h":0.45}'),
  ('scarp_3','box','mat.brick',0,'','{"y":0.95,"w":0.98,"d":0.9,"h":0.4}'),
  ('cordon','box','mat.cloth',1,'','{"y":1.35,"w":0.98,"d":0.84,"h":0.1}'),
  ('parapet','box','mat.turf',0,'','{"y":1.45,"z":0.18,"w":0.98,"d":0.42,"h":0.3}'),
  ('banquette','box','mat.packed_earth',0,'','{"y":1.45,"z":-0.22,"w":0.98,"d":0.36,"h":0.1}'),
  ('banquette_cloth','box','mat.cloth',1,'','{"y":1.55,"z":-0.26,"w":0.98,"d":0.18,"h":0.02}'),
  ('team_front','box','mat.cloth',1,'','{"y":0.9,"z":0.465,"w":0.98,"d":0.02,"h":0.5}'),
  ('team_back','box','mat.cloth',1,'','{"y":0.9,"z":-0.465,"w":0.98,"d":0.02,"h":0.5}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.palisade_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.palisade'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.palisade_gunpowder'), (SELECT id FROM def WHERE key='mat.brick'), 100);
