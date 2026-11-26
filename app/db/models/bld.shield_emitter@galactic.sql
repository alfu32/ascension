-- bld.shield_emitter@galactic: Shield emitter (mesh.shield_emitter_galactic)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.shield_emitter_galactic','Shield emitter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.shield_emitter_galactic';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.graphite',0,'','{"w":0.92,"d":0.9,"h":0.05}'),
  ('skirt','box','mat.white_metal',0,'','{"y":0.05,"w":0.6,"d":0.7,"h":0.2,"taper":0.35}'),
  ('mast','cone','mat.graphite',0,'','{"y":0.25,"w":0.3,"h":1.95,"segments":6}'),
  ('band','cylinder','mat.white_metal',1,'','{"y":0.66,"w":0.29,"h":0.6,"segments":6}'),
  ('halo_low','torus','mat.light_strip',0,'','{"y":0.55,"w":0.5,"t":0.04,"segments":12,"sides":4}'),
  ('halo_high','torus','mat.light_strip',0,'','{"y":1.45,"w":0.34,"t":0.04,"segments":12,"sides":4,"far":0}'),
  ('bar_low','box','mat.light_strip',0,'','{"y":0.27,"w":0.88,"d":0.05,"h":0.04}'),
  ('bar_high','box','mat.light_strip',0,'','{"y":1.86,"w":0.6,"d":0.05,"h":0.04}'),
  ('node','dome','mat.light_strip',0,'','{"y":2.2,"w":0.22,"h":0.2,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.shield_emitter_galactic'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.shield_emitter'), NULL, (SELECT id FROM def WHERE key='age.galactic'), (SELECT id FROM def WHERE key='mesh.shield_emitter_galactic'), (SELECT id FROM def WHERE key='mat.graphite'), 100);
