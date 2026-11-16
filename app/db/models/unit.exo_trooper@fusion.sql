-- unit.exo_trooper@fusion: Exo-trooper (mesh.exo_trooper_fusion)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.exo_trooper_fusion','Exo-trooper');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.exo_trooper_fusion';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":5,"hold_right":10,"build":1.35,"h":1.04,"skin":"#dfe3e2","hair":"#30363c","legs":"#4d545c","stride":24}'),
  ('helmet','box','mat.white_composite',0,'head','{"y":-0.22,"w":0.26,"d":0.27,"h":0.25,"taper":0.18}'),
  ('visor','box','mat.light_strip',0,'head','{"y":-0.13,"z":0.125,"w":0.2,"d":0.02,"h":0.05}'),
  ('yoke','box','mat.white_composite',1,'body','{"y":0.7,"w":0.66,"d":0.3,"h":0.14,"taper":0.2}'),
  ('pack','box','mat.graphite',0,'back','{"y":-0.26,"z":-0.08,"w":0.34,"d":0.2,"h":0.62,"taper":0.15}'),
  ('vents','box','mat.plasma',0,'pack','{"y":0.4,"z":-0.105,"w":0.2,"d":0.02,"h":0.06,"far":0}'),
  ('receiver','box','mat.graphite',0,'right_hand','{"y":-0.24,"w":0.08,"d":0.12,"h":0.56}'),
  ('barrel','cylinder','mat.graphite',0,'receiver','{"y":0.56,"w":0.06,"h":0.26,"segments":4}'),
  ('spark','cone','mat.flash',0,'barrel','{"y":0.27,"w":0.1,"h":0.1,"segments":3,"flash":2,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.28,"w":0.28,"h":0.28,"segments":6,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.exo_trooper_fusion'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.exo_trooper'), NULL, (SELECT id FROM def WHERE key='age.fusion'), (SELECT id FROM def WHERE key='mesh.exo_trooper_fusion'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
