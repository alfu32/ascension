-- unit.missile_team@information: Missile team (mesh.missile_team_information)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.missile_team_information','Missile team');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.missile_team_information';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('loader','figure','mat.cloth',1,'','{"x":0.48,"z":-0.3,"yaw":-25,"attack":7,"hold_right":30,"hold_left":30,"legs":"#5f5c4b"}'),
  ('loader_helmet','dome','mat.khaki',0,'loader.head','{"y":-0.11,"w":0.28,"h":0.15,"segments":6}'),
  ('loader_helmet_goggles','box','mat.glass',0,'loader.head','{"y":-0.03,"z":0.11,"w":0.2,"d":0.06,"h":0.055,"tint":"#506a88","far":0}'),
  ('loader_helmet_ears','cylinder','mat.composite',0,'loader.head','{"y":-0.12,"x":-0.12,"w":0.065,"h":0.24,"segments":4,"roll":90,"far":0}'),
  ('spare','cylinder','mat.nato_green',0,'loader.back','{"y":-0.36,"z":-0.06,"w":0.13,"h":0.7,"segments":6}'),
  ('gunner','figure','mat.cloth',1,'','{"x":-0.12,"y":-0.1,"z":-0.62,"seat":1,"attack":7,"hold_right":70,"hold_left":70,"legs":"#5f5c4b","far":0}'),
  ('gunner_helmet','dome','mat.khaki',0,'gunner.head','{"y":-0.11,"w":0.28,"h":0.15,"segments":6,"far":0}'),
  ('gunner_helmet_goggles','box','mat.glass',0,'gunner.head','{"y":-0.03,"z":0.11,"w":0.2,"d":0.06,"h":0.055,"tint":"#506a88","far":0}'),
  ('gunner_helmet_ears','cylinder','mat.composite',0,'gunner.head','{"y":-0.12,"x":-0.12,"w":0.065,"h":0.24,"segments":4,"roll":90,"far":0}'),
  ('tripod_front','box','mat.composite',0,'','{"x":-0.12,"z":0.15,"w":0.035,"d":0.035,"h":0.42,"pitch":-28}'),
  ('tripod_w','box','mat.composite',0,'','{"x":-0.36,"z":-0.36,"w":0.035,"d":0.035,"h":0.44,"roll":28,"pitch":26,"far":0}'),
  ('tripod_e','box','mat.composite',0,'','{"x":0.12,"z":-0.36,"w":0.035,"d":0.035,"h":0.44,"roll":-28,"pitch":26,"far":0}'),
  ('tube','cylinder','mat.nato_green',1,'','{"x":-0.12,"y":0.5,"z":-0.55,"w":0.14,"h":1,"segments":7,"pitch":88}'),
  ('sight','box','mat.composite',0,'','{"x":0.05,"y":0.46,"z":-0.3,"w":0.14,"d":0.26,"h":0.16}'),
  ('spark','cone','mat.flash',0,'tube','{"y":1,"w":0.132,"h":0.132,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.352,"w":0.396,"h":0.352,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":1,"w":0.077,"h":0.33,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-0.88,"w":0.22,"h":0.9,"segments":5,"far":0}'),
  ('blast','cone','mat.flash',0,'tube','{"y":0,"w":0.12,"h":0.12,"segments":3,"pitch":180,"flash":1,"far":0}'),
  ('blast_cloud','cone','mat.canvas',0,'blast','{"y":0.4,"w":0.42,"h":0.4,"segments":5,"pitch":180,"far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.missile_team_information'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.missile_team'), NULL, (SELECT id FROM def WHERE key='age.information'), (SELECT id FROM def WHERE key='mesh.missile_team_information'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
