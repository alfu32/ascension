-- bld.shield_emitter@starfaring: Shield emitter (mesh.shield_emitter_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.shield_emitter_starfaring','Shield emitter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.shield_emitter_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.graphite',0,'','{"w":0.92,"d":0.88,"h":0.06}'),
  ('skirt','box','mat.white_composite',0,'','{"y":0.06,"w":0.66,"d":0.66,"h":0.18,"taper":0.25}'),
  ('mast','cylinder','mat.white_composite',0,'','{"y":0.24,"w":0.24,"h":1.75,"taper":0.3,"segments":8}'),
  ('band','cylinder','mat.white_composite',1,'','{"y":0.78,"w":0.29,"h":0.7,"segments":8}'),
  ('coil_low','torus','mat.composite_panel',0,'','{"y":0.6,"w":0.44,"t":0.06,"segments":12,"sides":4}'),
  ('coil_high','torus','mat.composite_panel',0,'','{"y":1.5,"w":0.38,"t":0.05,"segments":12,"sides":4,"far":0}'),
  ('bar_low','box','mat.light_strip',0,'','{"y":0.28,"w":0.86,"d":0.05,"h":0.04}'),
  ('bar_high','box','mat.light_strip',0,'','{"y":1.82,"w":0.8,"d":0.05,"h":0.04}'),
  ('spine','box','mat.light_strip',0,'','{"y":0.3,"w":0.05,"d":0.27,"h":1.5,"far":0}'),
  ('node','dome','mat.light_strip',0,'','{"y":1.99,"w":0.26,"h":0.16,"segments":8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.shield_emitter_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.shield_emitter'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.shield_emitter_starfaring'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
