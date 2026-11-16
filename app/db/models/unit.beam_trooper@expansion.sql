-- unit.beam_trooper@expansion: Beam Trooper (mesh.beam_trooper_expansion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.beam_trooper_expansion','Beam Trooper');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.beam_trooper_expansion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"skin":"#d6dadc","hair":"#30363c","legs":"#3f4650"}'),
  ('helmet','box','mat.white_composite',0,'head','{"y":-0.18,"w":0.24,"d":0.28,"h":0.27,"taper":0.3}'),
  ('crest','box','mat.white_composite',1,'helmet','{"y":0.2,"z":-0.02,"w":0.04,"d":0.3,"h":0.12,"taper":0.3,"far":0}'),
  ('visor','box','mat.light_strip',0,'head','{"y":-0.1,"z":0.125,"w":0.22,"d":0.03,"h":0.05}'),
  ('ring','torus','mat.graphite',0,'back','{"y":-0.18,"z":-0.16,"w":0.5,"t":0.07,"segments":6,"sides":3,"roll":90,"yaw":90,"far":0}'),
  ('rifle','box','mat.white_composite',0,'right_hand','{"y":-0.22,"w":0.07,"d":0.11,"h":0.66}'),
  ('coils','cylinder','mat.plasma',0,'rifle','{"y":0.42,"w":0.11,"h":0.04,"segments":4,"far":0}'),
  ('spark','cone','mat.light_strip',0,'rifle','{"y":0.67,"w":0.12,"h":0.1,"segments":3,"flash":1,"far":0}'),
  ('beam','cylinder','mat.light_strip',0,'spark','{"y":0.1,"w":0.035,"h":1,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.beam_trooper_expansion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.beam_trooper'), NULL, (SELECT id FROM def WHERE key='age.expansion'), (SELECT id FROM def WHERE key='mesh.beam_trooper_expansion'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
