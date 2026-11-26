-- bld.shield_gate@starfaring: Shield gate (mesh.shield_gate_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.shield_gate_starfaring','Shield gate');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.shield_gate_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bed','box','mat.graphite',0,'','{"w":2.94,"d":0.88,"h":0.05}'),
  ('sill','box','mat.white_composite',1,'','{"y":0.05,"w":1.1,"d":0.86,"h":0.07}'),
  ('footing_l','box','mat.white_composite',0,'','{"x":-1.28,"y":0.05,"w":0.34,"d":0.66,"h":0.2,"taper":0.25}'),
  ('footing_r','box','mat.white_composite',0,'','{"x":1.28,"y":0.05,"w":0.34,"d":0.66,"h":0.2,"taper":0.25}'),
  ('mast_l','cylinder','mat.white_composite',0,'','{"x":-1.28,"y":0.25,"w":0.26,"h":2.0,"segments":8,"taper":0.3}'),
  ('band_l','box','mat.white_composite',1,'','{"x":-1.28,"y":1.0,"w":0.3,"d":0.34,"h":0.26}'),
  ('jamb_l','box','mat.light_strip',0,'','{"x":-1.1904000000000001,"y":0.3,"w":0.04,"d":0.1,"h":1.7}'),
  ('mast_r','cylinder','mat.white_composite',0,'','{"x":1.28,"y":0.25,"w":0.26,"h":2.0,"segments":8,"taper":0.3}'),
  ('band_r','box','mat.white_composite',1,'','{"x":1.28,"y":1.0,"w":0.3,"d":0.34,"h":0.26}'),
  ('jamb_r','box','mat.light_strip',0,'','{"x":1.1904000000000001,"y":0.3,"w":0.04,"d":0.1,"h":1.7}'),
  ('lintel','box','mat.light_strip',0,'','{"y":2.02,"w":2.5,"d":0.08,"h":0.06}'),
  ('threshold','box','mat.light_strip',0,'','{"y":0.12,"w":2.3,"d":0.06,"h":0.03}'),
  ('node_l','dome','mat.light_strip',0,'','{"x":-1.28,"y":2.25,"w":0.24,"h":0.16,"segments":8}'),
  ('node_r','dome','mat.light_strip',0,'','{"x":1.28,"y":2.25,"w":0.24,"h":0.16,"segments":8}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.shield_gate_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.shield_gate'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.shield_gate_starfaring'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
