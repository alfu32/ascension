-- bld.checkpoint@industrial: Trench checkpoint (mesh.checkpoint_industrial)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.checkpoint_industrial','Trench checkpoint');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.checkpoint_industrial';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('sandbags_l','box','mat.sandbag',0,'','{"x":-0.38,"w":0.24,"d":0.6,"h":0.6,"taper":0.15}'),
  ('sandbags_r','box','mat.sandbag',0,'','{"x":0.38,"w":0.24,"d":0.6,"h":0.6,"taper":0.15}'),
  ('boom','box','mat.sawn_timber',0,'','{"x":0.0,"y":0.75,"w":0.95,"d":0.06,"h":0.06}'),
  ('boom_stripe','box','mat.signal_red',0,'','{"x":-0.2,"y":0.745,"w":0.18,"d":0.07,"h":0.07,"repeat":2,"dx":0.4}'),
  ('post','box','mat.sawn_timber',0,'','{"x":0.42,"w":0.08,"d":0.08,"h":0.8}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.38,"y":0.6,"h":0.8,"fw":0.34,"fh":0.22}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.checkpoint_industrial'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.checkpoint'), NULL, (SELECT id FROM def WHERE key='age.industrial'), (SELECT id FROM def WHERE key='mesh.checkpoint_industrial'), (SELECT id FROM def WHERE key='mat.sandbag'), 100);
