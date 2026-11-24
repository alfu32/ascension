-- bld.checkpoint@information: Barrier checkpoint (mesh.checkpoint_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.checkpoint_information','Barrier checkpoint');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.checkpoint_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('block_l','box','mat.precast',0,'','{"x":-0.4,"w":0.2,"d":0.5,"h":0.7,"taper":0.45}'),
  ('booth','box','mat.smart_glass',0,'','{"x":0.36,"z":-0.22,"w":0.28,"d":0.3,"h":0.85}'),
  ('booth_roof','flat_roof','mat.cloth',1,'','{"x":0.36,"y":0.85,"z":-0.22,"w":0.28,"d":0.3,"t":0.06,"overhang":0.05}'),
  ('boom','box','mat.white_metal',0,'','{"x":-0.02,"y":0.7,"w":0.8,"d":0.05,"h":0.05}'),
  ('boom_stripe','box','mat.signal_red',0,'','{"x":-0.22,"y":0.695,"w":0.14,"d":0.06,"h":0.06,"repeat":2,"dx":0.36}'),
  ('camera','box','mat.steel',0,'','{"x":0.36,"y":0.91,"z":-0.1,"w":0.08,"d":0.12,"h":0.06,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.checkpoint_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.checkpoint'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.checkpoint_information'), (SELECT id FROM def WHERE key='mat.precast'), 100);
