-- unit.orbital_bomber@starfaring: Orbital Bombers (mesh.orbital_bomber_starfaring)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.orbital_bomber_starfaring','Orbital Bombers');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.orbital_bomber_starfaring';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('body','box','mat.steel',0,'','{"y":0.3,"w":0.62,"d":1.9,"h":0.34,"taper":0.2,"tint":"#9aa3ae"}'),
  ('nose','box','mat.steel',0,'','{"y":0.33,"z":1.15,"w":0.4,"d":0.45,"h":0.22,"taper":0.5,"tint":"#c3cbd6"}'),
  ('canopy','box','mat.steel',0,'','{"y":0.62,"z":0.7,"w":0.3,"d":0.36,"h":0.1,"taper":0.4,"tint":"#3b5a78","far":0}'),
  ('wing','box','mat.steel',1,'','{"y":0.26,"z":-0.2,"w":2.1,"d":0.85,"h":0.07,"taper":0.45}'),
  ('bay','box','mat.iron',0,'','{"y":0.16,"z":0.1,"w":0.42,"d":1.0,"h":0.16,"tint":"#5d6470"}'),
  ('ordnance','cylinder','mat.iron',0,'','{"x":-0.6,"y":0.12,"z":-0.1,"w":0.14,"h":0.5,"segments":6,"pitch":90,"repeat":2,"dx":1.2,"tint":"#3d434c","far":0}'),
  ('engine','cylinder','mat.iron',0,'','{"x":-0.18,"y":0.32,"z":-1.05,"w":0.24,"h":0.3,"segments":8,"pitch":90,"repeat":2,"dx":0.36,"tint":"#4a5260","far":0}'),
  ('drive','wheel','mat.glow',0,'','{"x":-0.18,"y":0.32,"z":-1.22,"yaw":90,"w":0.2,"t":0.04,"spokes":3,"segments":8,"repeat":2,"dx":0.36,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.orbital_bomber_starfaring'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.orbital_bomber'), NULL, (SELECT id FROM def WHERE key='age.starfaring'), (SELECT id FROM def WHERE key='mesh.orbital_bomber_starfaring'), (SELECT id FROM def WHERE key='mat.steel'), 100);
