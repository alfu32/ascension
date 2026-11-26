-- bld.shield_emitter@abundance: Shield emitter (mesh.shield_emitter_abundance)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.shield_emitter_abundance','Shield emitter');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.shield_emitter_abundance';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.composite_panel',0,'','{"w":0.92,"d":0.9,"h":0.06}'),
  ('skirt','cylinder','mat.white_metal',0,'','{"y":0.06,"w":0.72,"h":0.16,"taper":0.3,"segments":10}'),
  ('mast','box','mat.white_metal',0,'','{"y":0.22,"w":0.16,"d":0.3,"h":1.8,"taper":0.35}'),
  ('band','box','mat.white_metal',1,'','{"y":0.75,"w":0.2,"d":0.34,"h":0.6}'),
  ('fins','box','mat.composite_panel',0,'','{"x":-0.3,"y":0.45,"w":0.06,"d":0.12,"h":1.2,"repeat":2,"dx":0.6}'),
  ('bar_low','box','mat.plasma',0,'','{"y":0.26,"w":0.88,"d":0.05,"h":0.04}'),
  ('bar_mid','box','mat.plasma',0,'','{"y":1.05,"w":0.7,"d":0.04,"h":0.03,"far":0}'),
  ('bar_high','box','mat.plasma',0,'','{"y":1.84,"w":0.84,"d":0.05,"h":0.04}'),
  ('node','cylinder','mat.plasma',0,'','{"y":2.02,"w":0.2,"h":0.14,"taper":0.5,"segments":6}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.shield_emitter_abundance'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.shield_emitter'), NULL, (SELECT id FROM def WHERE key='age.abundance'), (SELECT id FROM def WHERE key='mesh.shield_emitter_abundance'), (SELECT id FROM def WHERE key='mat.white_metal'), 100);
