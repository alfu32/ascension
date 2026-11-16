-- unit.space_freighter@starfaring: Space Freighters (mesh.space_freighter_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.space_freighter_starfaring','Space Freighters');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.space_freighter_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('keel','box','mat.steel',0,'','{"y":0.12,"w":0.5,"d":3.2,"h":0.26,"taper":0.1,"tint":"#8e97a3"}'),
  ('container','box','mat.steel',0,'keel','{"z":-0.95,"y":0.38,"w":0.9,"d":0.78,"h":0.6,"repeat":3,"dz":0.9,"tint":"#b98a4a"}'),
  ('band','box','mat.steel',1,'keel','{"z":-0.95,"y":0.38,"w":0.94,"d":0.3,"h":0.62,"repeat":3,"dz":0.9}'),
  ('fin','box','mat.steel',1,'keel','{"y":0.16,"z":-1.3,"w":1.9,"d":0.55,"h":0.06,"taper":0.3}'),
  ('bridge','box','mat.steel',0,'keel','{"y":0.4,"z":1.45,"w":0.56,"d":0.45,"h":0.36,"taper":0.25,"tint":"#c3cbd6","far":0}'),
  ('window','box','mat.steel',0,'keel','{"y":0.5,"z":1.68,"w":0.4,"d":0.04,"h":0.1,"tint":"#3b5a78","far":0}'),
  ('engine','cylinder','mat.iron',0,'keel','{"x":-0.3,"y":0.2,"z":-1.75,"w":0.32,"h":0.4,"segments":8,"pitch":90,"repeat":2,"dx":0.6,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'keel','{"x":-0.3,"y":0.2,"z":-1.97,"yaw":90,"w":0.26,"t":0.05,"spokes":3,"segments":8,"repeat":2,"dx":0.6,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.space_freighter_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.space_freighter'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.space_freighter_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
