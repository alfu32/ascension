-- unit.satellite@starfaring: Satellites (mesh.satellite_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.satellite_starfaring','Satellites');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.satellite_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('bus','box','mat.steel',0,'','{"y":0.3,"w":0.36,"d":0.36,"h":0.42,"tint":"#c9a64a"}'),
  ('band','box','mat.steel',1,'bus','{"y":0.5,"w":0.38,"d":0.38,"h":0.08}'),
  ('mast','cylinder','mat.iron',0,'bus','{"y":0.3,"w":0.05,"h":1.7,"segments":6,"roll":90,"tint":"#7c8592"}'),
  ('wing','box','mat.steel',0,'bus','{"x":-0.62,"y":0.3,"w":0.82,"d":0.42,"h":0.03,"repeat":2,"dx":1.24,"tint":"#24365a"}'),
  ('dish','cone','mat.white_metal',0,'bus','{"y":0.02,"w":0.34,"h":0.14,"segments":10,"pitch":180,"tint":"#d8dde3"}'),
  ('feed','cylinder','mat.iron',0,'bus','{"y":-0.08,"w":0.04,"h":0.12,"segments":5,"far":0}'),
  ('thruster','wheel','mat.glow',0,'bus','{"y":0.3,"z":-0.2,"yaw":90,"w":0.16,"t":0.03,"spokes":3,"segments":6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.satellite_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.satellite'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.satellite_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
