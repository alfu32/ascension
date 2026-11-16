-- unit.carrier@starfaring: Carriers (mesh.carrier_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.carrier_starfaring','Carriers');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.carrier_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.25,"w":1.5,"d":4.0,"h":0.7,"taper":0.08,"tint":"#9aa4b2"}'),
  ('deck','box','mat.steel',0,'','{"y":0.95,"z":0.1,"w":1.7,"d":3.8,"h":0.08,"tint":"#4b525c"}'),
  ('stripe','box','mat.steel',1,'','{"y":1.03,"z":0.1,"w":0.12,"d":3.4,"h":0.02,"far":0}'),
  ('hangar','box','mat.iron',0,'','{"y":0.6,"z":1.98,"w":1.0,"d":0.06,"h":0.32,"tint":"#1e2228","far":0}'),
  ('island','box','mat.steel',0,'','{"x":0.62,"y":1.03,"z":-0.3,"w":0.32,"d":0.9,"h":0.5,"taper":0.15,"tint":"#c3cbd6"}'),
  ('mast','cylinder','mat.iron',1,'','{"x":0.62,"y":1.5,"z":-0.3,"w":0.08,"h":0.35,"segments":6}'),
  ('wing','box','mat.steel',1,'','{"y":0.3,"z":-1.2,"w":2.6,"d":0.8,"h":0.07,"taper":0.3}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.45,"y":0.45,"z":-2.1,"w":0.42,"h":0.4,"segments":8,"pitch":90,"repeat":3,"dx":0.45,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.45,"y":0.45,"z":-2.32,"yaw":90,"w":0.34,"t":0.05,"spokes":3,"segments":8,"repeat":3,"dx":0.45,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.carrier_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.carrier'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.carrier_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
