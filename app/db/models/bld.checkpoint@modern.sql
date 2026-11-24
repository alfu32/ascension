-- bld.checkpoint@modern: Checkpoint (mesh.checkpoint_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.checkpoint_modern','Checkpoint');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.checkpoint_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.concrete',0,'','{"w":0.96,"d":0.86,"h":0.02}'),
  ('block_l','box','mat.concrete',0,'','{"x":-0.39,"w":0.2,"d":0.55,"h":0.6,"taper":0.3}'),
  ('hut','box','mat.concrete',0,'','{"x":0.35,"z":-0.22,"w":0.28,"d":0.3,"h":0.8,"windows":1,"window_w":0.12,"window_h":0.14,"window_y":0.5}'),
  ('hut_roof','flat_roof','mat.cloth',1,'','{"x":0.35,"y":0.8,"z":-0.22,"w":0.28,"d":0.3,"t":0.06,"overhang":0.04}'),
  ('boom','box','mat.white_metal',0,'','{"x":-0.02,"y":0.62,"w":0.8,"d":0.05,"h":0.05}'),
  ('boom_stripe','box','mat.signal_red',0,'','{"x":-0.22,"y":0.615,"w":0.14,"d":0.06,"h":0.06,"repeat":2,"dx":0.36}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.4,"y":0.6,"h":0.8,"fw":0.34,"fh":0.22}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.checkpoint_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.checkpoint'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.checkpoint_modern'), (SELECT id FROM def WHERE key='mat.concrete'), 100);
