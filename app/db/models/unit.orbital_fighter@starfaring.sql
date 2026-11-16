-- unit.orbital_fighter@starfaring: Orbital Fighters (mesh.orbital_fighter_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.orbital_fighter_starfaring','Orbital Fighters');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.orbital_fighter_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.steel',0,'','{"y":0.25,"w":0.36,"d":1.5,"h":0.22,"taper":0.3,"tint":"#b4bcc8"}'),
  ('nose','box','mat.steel',0,'','{"y":0.27,"z":0.95,"w":0.22,"d":0.45,"h":0.14,"taper":0.6,"tint":"#d2d8e0"}'),
  ('canopy','box','mat.steel',0,'','{"y":0.45,"z":0.35,"w":0.18,"d":0.4,"h":0.1,"taper":0.4,"tint":"#3b5a78","far":0}'),
  ('wing','box','mat.steel',1,'','{"y":0.22,"z":-0.25,"w":1.5,"d":0.6,"h":0.05,"taper":0.55}'),
  ('fin','box','mat.steel',0,'','{"y":0.45,"z":-0.5,"w":0.04,"d":0.42,"h":0.2,"taper":0.5,"tint":"#8e97a3"}'),
  ('engine','cylinder','mat.iron',0,'','{"y":0.25,"z":-0.82,"w":0.24,"h":0.22,"segments":8,"pitch":90,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"y":0.25,"z":-0.95,"yaw":90,"w":0.2,"t":0.04,"spokes":3,"segments":8,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.orbital_fighter_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.orbital_fighter'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.orbital_fighter_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
