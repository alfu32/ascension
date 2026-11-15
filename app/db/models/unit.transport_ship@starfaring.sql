-- unit.transport_ship@starfaring: Transport Ship (mesh.transport_ship_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.transport_ship_starfaring','Transport Ship');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.transport_ship_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('hull','box','mat.steel',0,'','{"y":0.2,"w":1.1,"d":3.0,"h":0.62,"taper":0.12,"tint":"#a6b0bd"}'),
  ('pod','box','mat.steel',0,'hull','{"x":-0.78,"y":0.05,"w":0.46,"d":1.9,"h":0.5,"taper":0.06,"repeat":2,"dx":1.56,"tint":"#7c8592"}'),
  ('spine','box','mat.steel',1,'hull','{"y":0.62,"z":-0.15,"w":0.42,"d":2.0,"h":0.14}'),
  ('bridge','box','mat.steel',0,'hull','{"y":0.62,"z":1.05,"w":0.62,"d":0.55,"h":0.3,"taper":0.15,"tint":"#c3cbd6","far":0}'),
  ('wing','box','mat.steel',1,'hull','{"y":0.22,"z":-0.75,"w":2.8,"d":0.7,"h":0.07,"taper":0.3}'),
  ('engine','cylinder','mat.iron',0,'hull','{"x":-0.42,"y":0.32,"z":-1.85,"w":0.38,"h":0.42,"segments":8,"pitch":90,"repeat":2,"dx":0.84,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'hull','{"x":-0.42,"y":0.17,"z":-1.9,"yaw":90,"w":0.3,"t":0.05,"spokes":3,"segments":8,"repeat":2,"dx":0.84,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.transport_ship_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.transport_ship'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.transport_ship_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
