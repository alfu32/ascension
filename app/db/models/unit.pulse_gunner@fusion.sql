-- unit.pulse_gunner@fusion: Pulse gunner (mesh.pulse_gunner_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.pulse_gunner_fusion','Pulse gunner');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.pulse_gunner_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"legs":"#4d545c"}'),
  ('helmet','dome','mat.white_composite',0,'head','{"y":-0.12,"w":0.3,"h":0.17,"segments":6}'),
  ('visor','box','mat.light_strip',0,'head','{"y":-0.07,"z":0.115,"w":0.2,"d":0.04,"h":0.045,"far":0}'),
  ('vest','box','mat.white_composite',0,'body','{"y":0.56,"z":0.03,"w":0.3,"d":0.2,"h":0.16,"far":0}'),
  ('cell','cylinder','mat.graphite',0,'back','{"y":-0.3,"z":-0.06,"w":0.2,"h":0.5,"segments":5}'),
  ('cell_glow','cylinder','mat.plasma',0,'cell','{"y":0.16,"w":0.21,"h":0.14,"segments":5,"far":0}'),
  ('rifle','box','mat.white_composite',0,'right_hand','{"y":-0.2,"w":0.06,"d":0.1,"h":0.46,"far":0}'),
  ('emitter','cylinder','mat.graphite',0,'rifle','{"y":0.46,"w":0.05,"h":0.16,"segments":3,"far":0}'),
  ('spark','cone','mat.plasma',0,'emitter','{"y":0.17,"w":0.1,"h":0.1,"segments":3,"flash":3,"far":0}'),
  ('bolt','cylinder','mat.plasma',0,'spark','{"y":0.45,"w":0.06,"h":0.4,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.pulse_gunner_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.pulse_gunner'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.pulse_gunner_fusion'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
