-- bld.jump_inhibitor@starfaring: Jump Inhibitor (mesh.jump_inhibitor_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.jump_inhibitor_starfaring','Jump Inhibitor');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.jump_inhibitor_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('pad','plinth','mat.concrete',0,'','{"w":2.75,"d":2.75,"h":0.22,"steps":2,"inset":0.12}'),
  ('deck','box','mat.graphite',0,'','{"y":0.22,"w":1.5,"d":1.5,"h":0.28,"taper":0.15}'),
  ('trim_south','box','mat.white_composite',1,'','{"y":0.22,"z":1.2,"w":2.3,"d":0.22,"h":0.06}'),
  ('trim_north','box','mat.white_composite',1,'','{"y":0.22,"z":-1.2,"w":2.3,"d":0.22,"h":0.06}'),
  ('mast','cylinder','mat.composite_panel',0,'','{"y":0.5,"w":0.5,"h":2.5,"taper":0.35,"segments":10}'),
  ('mast_band','cylinder','mat.white_composite',1,'','{"y":0.85,"w":0.56,"h":0.3,"segments":10}'),
  ('ring_low','torus','mat.white_composite',0,'','{"y":1.3,"w":1.7,"t":0.12,"segments":18,"sides":5}'),
  ('ring_mid','torus','mat.white_composite',0,'','{"y":1.85,"w":1.35,"t":0.11,"segments":16,"sides":5}'),
  ('ring_high','torus','mat.white_composite',0,'','{"y":2.35,"w":1,"t":0.1,"segments":14,"sides":5,"far":0}'),
  ('field','torus','mat.light_strip',0,'','{"y":1.6,"w":2.35,"t":0.05,"segments":24,"sides":4,"far":0}'),
  ('crown','dome','mat.light_strip',0,'','{"y":2.95,"w":0.34,"h":0.24,"segments":8}'),
  ('pylons_south','cylinder','mat.white_composite',0,'','{"x":-1,"y":0.22,"z":1,"w":0.2,"h":1.75,"taper":0.45,"segments":6,"repeat":2,"dx":2}'),
  ('pylons_north','cylinder','mat.white_composite',0,'','{"x":-1,"y":0.22,"z":-1,"w":0.2,"h":1.75,"taper":0.45,"segments":6,"repeat":2,"dx":2}'),
  ('tips_south','cone','mat.light_strip',0,'','{"x":-1,"y":1.97,"z":1,"w":0.14,"h":0.2,"segments":5,"repeat":2,"dx":2,"far":0}'),
  ('tips_north','cone','mat.light_strip',0,'','{"x":-1,"y":1.97,"z":-1,"w":0.14,"h":0.2,"segments":5,"repeat":2,"dx":2,"far":0}'),
  ('console','box','mat.white_composite',0,'','{"x":0.62,"y":0.22,"z":0.95,"w":0.5,"d":0.32,"h":0.34,"taper":0.2,"far":0}'),
  ('console_light','box','mat.signal_red',0,'','{"x":0.62,"y":0.56,"z":0.95,"w":0.08,"d":0.08,"h":0.06,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.jump_inhibitor_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.jump_inhibitor'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.jump_inhibitor_starfaring'), (SELECT id FROM def WHERE key='mat.composite_panel'), 100);
