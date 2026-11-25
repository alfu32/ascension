-- bld.checkpoint@fusion: Field gate (mesh.checkpoint_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.checkpoint_fusion','Field gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.checkpoint_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.white_composite',0,'','{"w":2.96,"d":0.86,"h":0.02}'),
  ('pylon_l','box','mat.white_metal',0,'','{"x":-1.41,"w":0.16,"d":0.24,"h":1.9,"taper":0.3}'),
  ('pylon_r','box','mat.white_metal',0,'','{"x":1.41,"w":0.16,"d":0.24,"h":1.9,"taper":0.3}'),
  ('head_l','box','mat.cloth',1,'','{"x":-1.39,"y":1.9,"w":0.2,"d":0.28,"h":0.1}'),
  ('head_r','box','mat.cloth',1,'','{"x":1.39,"y":1.9,"w":0.2,"d":0.28,"h":0.1}'),
  ('lintel','box','mat.light_strip',0,'','{"y":1.75,"w":2.68,"d":0.08,"h":0.06}'),
  ('jamb_l','box','mat.light_strip',0,'','{"x":-1.335,"y":0.1,"w":0.03,"d":0.08,"h":1.65}'),
  ('jamb_r','box','mat.light_strip',0,'','{"x":1.335,"y":0.1,"w":0.03,"d":0.08,"h":1.65}'),
  ('sill','box','mat.white_composite',0,'','{"w":2.98,"d":0.3,"h":0.08}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.checkpoint_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.checkpoint'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.checkpoint_fusion'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
