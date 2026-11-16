-- unit.shock_marine@abundance: Shock Marine (mesh.shock_marine_abundance)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.shock_marine_abundance','Shock Marine');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.shock_marine_abundance';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','figure','mat.cloth',1,'','{"attack":6,"build":1.45,"h":1.05,"skin":"#bfc5c9","hair":"#30363c","legs":"#3a4048","stride":22,"hold_left":40}'),
  ('helmet','box','mat.white_composite',0,'head','{"y":-0.22,"w":0.28,"d":0.3,"h":0.28,"taper":0.12}'),
  ('visor','box','mat.plasma',0,'head','{"y":-0.1,"z":0.14,"w":0.2,"d":0.02,"h":0.04}'),
  ('plate','box','mat.white_composite',1,'body','{"y":0.46,"z":0.04,"w":0.44,"d":0.3,"h":0.3,"taper":0.12,"far":0}'),
  ('pauldrons','box','mat.graphite',0,'body','{"x":-0.29,"y":0.62,"w":0.22,"d":0.28,"h":0.16,"taper":0.35,"repeat":2,"dx":0.58}'),
  ('shield','shield','mat.white_composite',0,'left_hand','{"y":0.02,"z":0.08,"w":0.46,"h":0.72,"t":0.05,"boss":0,"point":0,"rim":"#3a4048"}'),
  ('shield_stripe','box','mat.white_composite',1,'shield','{"y":-0.1,"z":0.03,"w":0.1,"d":0.02,"h":0.5,"far":0}'),
  ('blaster','box','mat.graphite',0,'right_hand','{"y":-0.16,"w":0.1,"d":0.14,"h":0.42}'),
  ('spark','cone','mat.plasma',0,'blaster','{"y":0.43,"w":0.14,"h":0.12,"segments":3,"flash":3,"far":0}'),
  ('bolt','cylinder','mat.plasma',0,'spark','{"y":0.4,"w":0.08,"h":0.4,"segments":3,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.shock_marine_abundance'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.shock_marine'), NULL, (SELECT id FROM def WHERE key='age.abundance'), (SELECT id FROM def WHERE key='mesh.shock_marine_abundance'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
